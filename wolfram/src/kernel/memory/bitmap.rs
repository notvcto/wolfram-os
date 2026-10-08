//! Boot-time physical frame allocator. One bit per 4 KiB frame.
//! The fixed bitmap manages up to 4 GiB of the RAM bank containing the kernel.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::boot_info::{BootInfo, MemoryRegion, MEMORY_RESERVED, MEMORY_USABLE};

const PAGE_SIZE: usize = 4096;
const WORDS: usize = 16_384;
const MAX_FRAMES: usize = WORDS * 64;

#[derive(Clone, Copy)]
pub struct MemoryStats {
    pub start: usize,
    pub end: usize,
    pub free_pages: usize,
}

struct State {
    // A set bit is unavailable. `permanent` prevents freeing kernel, firmware,
    // device-tree, and other reserved frames through the public API.
    used: [u64; WORDS],
    permanent: [u64; WORDS],
    base: usize,
    frames: usize,
    free: usize,
    initialized: bool,
}

struct LockedState {
    lock: AtomicBool,
    state: UnsafeCell<State>,
}

// All access to state is serialized by lock, including boot-time setup.
unsafe impl Sync for LockedState {}

static ALLOCATOR: LockedState = LockedState {
    lock: AtomicBool::new(false),
    state: UnsafeCell::new(State {
        used: [u64::MAX; WORDS],
        permanent: [u64::MAX; WORDS],
        base: 0,
        frames: 0,
        free: 0,
        initialized: false,
    }),
};

impl LockedState {
    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        struct Unlock<'a>(&'a AtomicBool);
        impl Drop for Unlock<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _unlock = Unlock(&self.lock);
        // SAFETY: the lock is held until the closure returns.
        f(unsafe { &mut *self.state.get() })
    }
}

impl State {
    fn set_used(&mut self, index: usize) {
        let bit = 1u64 << (index % 64);
        let word = &mut self.used[index / 64];
        if *word & bit == 0 {
            *word |= bit;
            self.free -= 1;
        }
    }

    fn reserve(&mut self, region: MemoryRegion) {
        let start = region.start.max(self.base as u64);
        let end = region.end.min((self.base + self.frames * PAGE_SIZE) as u64);
        if start >= end {
            return;
        }
        let first = ((start as usize) - self.base) / PAGE_SIZE;
        let last = ((end as usize) - self.base).div_ceil(PAGE_SIZE);
        for index in first..last {
            self.set_used(index);
            self.permanent[index / 64] |= 1u64 << (index % 64);
        }
    }

    fn release(&mut self, region: MemoryRegion) {
        let start = region.start.max(self.base as u64);
        let end = region.end.min((self.base + self.frames * PAGE_SIZE) as u64);
        if start >= end {
            return;
        }
        // A page is usable only if the entire page lies in the supplied range.
        let first = ((start as usize) - self.base).div_ceil(PAGE_SIZE);
        let last = ((end as usize) - self.base) / PAGE_SIZE;
        for index in first..last {
            let bit = 1u64 << (index % 64);
            let word = &mut self.used[index / 64];
            if *word & bit != 0 {
                *word &= !bit;
                self.permanent[index / 64] &= !bit;
                self.free += 1;
            }
        }
    }

    fn stats(&self) -> MemoryStats {
        MemoryStats {
            start: self.base,
            end: self.base + self.frames * PAGE_SIZE,
            free_pages: self.free,
        }
    }
}

/// Initialize from a validated boot handoff. A single RAM bank of up to 4 GiB
/// is managed for Phase 1. Reserved ranges override usable ranges.
pub fn init_from_boot_info(info: &BootInfo) -> MemoryStats {
    info.validate()
        .unwrap_or_else(|e| panic!("physical memory detection: {}", e));
    // SAFETY: the architecture boot contract guarantees the array's lifetime
    // and identity mapping until this function completes.
    let regions = unsafe { info.regions() };
    let mut bank = None;
    for &region in regions {
        assert!(region.start < region.end, "invalid boot memory range");
        assert!(
            region.kind == MEMORY_USABLE || region.kind == MEMORY_RESERVED,
            "invalid boot memory kind"
        );
        if region.kind != MEMORY_USABLE {
            continue;
        }
        let contains_kernel = region.start <= info.kernel_start && region.end >= info.kernel_end;
        match bank {
            None => bank = Some(region),
            Some(current) => {
                let current_contains =
                    current.start <= info.kernel_start && current.end >= info.kernel_end;
                if (contains_kernel && !current_contains)
                    || (contains_kernel == current_contains
                        && region.end - region.start > current.end - current.start)
                {
                    bank = Some(region);
                }
            }
        }
    }
    let bank = bank.expect("no usable RAM bank in boot memory map");
    let start = usize::try_from(bank.start).expect("RAM address does not fit usize");
    let end = usize::try_from(bank.end).expect("RAM end does not fit usize");
    let base = start
        .checked_add(PAGE_SIZE - 1)
        .expect("RAM address overflow")
        & !(PAGE_SIZE - 1);
    let managed_end = end.min(
        base.checked_add(MAX_FRAMES * PAGE_SIZE)
            .expect("RAM span overflow"),
    ) & !(PAGE_SIZE - 1);
    assert!(managed_end > base, "RAM bank has no complete pages");
    if bank.start <= info.kernel_start && bank.end >= info.kernel_end {
        assert!(
            info.kernel_end <= managed_end as u64,
            "kernel lies outside bitmap capacity"
        );
    }
    let frames = (managed_end - base) / PAGE_SIZE;

    ALLOCATOR.with(|state| {
        assert!(!state.initialized, "physical allocator initialized twice");
        state.base = base;
        state.frames = frames;
        // The bitmap starts unavailable. Only explicit usable ranges can
        // release complete frames, then all reservations are reapplied.
        for &region in regions {
            if region.kind == MEMORY_USABLE {
                state.release(region);
            }
        }
        for &region in regions {
            if region.kind == MEMORY_RESERVED {
                state.reserve(region);
            }
        }
        state.reserve(MemoryRegion::reserved(info.kernel_start, info.kernel_end));
        state.reserve(MemoryRegion::reserved(
            info as *const BootInfo as u64,
            (info as *const BootInfo as u64)
                .checked_add(core::mem::size_of::<BootInfo>() as u64)
                .expect("boot information range overflow"),
        ));
        state.reserve(MemoryRegion::reserved(
            info.memory_regions,
            info.memory_regions
                .checked_add(regions.len() as u64 * core::mem::size_of::<MemoryRegion>() as u64)
                .expect("boot memory map range overflow"),
        ));
        if info.framebuffer.base != 0 && info.framebuffer.size != 0 {
            state.reserve(MemoryRegion::reserved(
                info.framebuffer.base,
                info.framebuffer
                    .base
                    .checked_add(info.framebuffer.size)
                    .expect("framebuffer range overflow"),
            ));
        }
        // Keep the first frame permanently unavailable. This also catches
        // accidental use of a bank boundary as a freeable frame.
        state.reserve(MemoryRegion::reserved(
            base as u64,
            (base + PAGE_SIZE) as u64,
        ));
        state.initialized = true;
        state.stats()
    })
}

/// Returns the physical address of a free 4 KiB frame.
pub fn allocate_frame() -> Option<usize> {
    ALLOCATOR.with(|state| {
        assert!(state.initialized, "physical allocator used before init");
        if state.free == 0 {
            return None;
        }
        for word_index in 0..state.frames.div_ceil(64) {
            let available = !state.used[word_index];
            if available == 0 {
                continue;
            }
            let index = word_index * 64 + available.trailing_zeros() as usize;
            if index < state.frames {
                state.set_used(index);
                return Some(state.base + index * PAGE_SIZE);
            }
        }
        None
    })
}

pub fn free_frame(address: usize) -> Result<(), &'static str> {
    ALLOCATOR.with(|state| {
        if !state.initialized {
            return Err("physical allocator used before init");
        }
        if address < state.base
            || address >= state.base + state.frames * PAGE_SIZE
            || !address.is_multiple_of(PAGE_SIZE)
        {
            return Err("frame outside managed RAM or unaligned");
        }
        let index = (address - state.base) / PAGE_SIZE;
        let bit = 1u64 << (index % 64);
        if state.permanent[index / 64] & bit != 0 {
            return Err("frame is reserved");
        }
        let word = &mut state.used[index / 64];
        if *word & bit == 0 {
            return Err("frame is already free");
        }
        *word &= !bit;
        state.free += 1;
        Ok(())
    })
}
