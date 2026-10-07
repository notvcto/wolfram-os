//! Boot-time physical frame allocator. One bit per 4 KiB frame.
//! The fixed bitmap manages up to 4 GiB of the RAM bank containing the kernel.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::device_tree::{DeviceTree, Region};

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
        while self.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        struct Unlock<'a>(&'a AtomicBool);
        impl Drop for Unlock<'_> {
            fn drop(&mut self) { self.0.store(false, Ordering::Release); }
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

    fn reserve(&mut self, region: Region) {
        let start = region.start.max(self.base as u64);
        let end = region.end.min((self.base + self.frames * PAGE_SIZE) as u64);
        if start >= end { return; }
        let first = ((start as usize) - self.base) / PAGE_SIZE;
        let last = (((end as usize) - self.base) + PAGE_SIZE - 1) / PAGE_SIZE;
        for index in first..last {
            self.set_used(index);
            self.permanent[index / 64] |= 1u64 << (index % 64);
        }
    }

    fn stats(&self) -> MemoryStats {
        MemoryStats { start: self.base, end: self.base + self.frames * PAGE_SIZE, free_pages: self.free }
    }
}

/// Initialize from the firmware FDT. Only the RAM bank containing the kernel
/// is managed; unsupported layouts fail rather than risking reserved memory.
pub fn init(device_tree: usize) -> MemoryStats {
    // SAFETY: the SBI boot contract supplies a mapped FDT in a1.
    let tree = unsafe { DeviceTree::from_ptr(device_tree) }
        .unwrap_or_else(|e| panic!("physical memory detection: {}", e));
    extern "C" { static __kernel_start: u8; static __kernel_end: u8; }
    let kernel_start = &raw const __kernel_start as usize;
    let kernel_end = &raw const __kernel_end as usize;
    let mut bank = None;
    tree.memory_regions(|region| {
        if region.start <= kernel_start as u64 && region.end >= kernel_end as u64 {
            bank = Some(region);
        }
    }).unwrap_or_else(|e| panic!("physical memory detection: {}", e));
    let bank = bank.expect("no FDT RAM bank contains the kernel");
    let start = usize::try_from(bank.start).expect("RAM address does not fit usize");
    let end = usize::try_from(bank.end).expect("RAM end does not fit usize");
    let base = start.checked_add(PAGE_SIZE - 1).expect("RAM address overflow") & !(PAGE_SIZE - 1);
    let managed_end = end.min(base.checked_add(MAX_FRAMES * PAGE_SIZE).expect("RAM span overflow")) & !(PAGE_SIZE - 1);
    assert!(kernel_end < managed_end, "kernel lies outside bitmap capacity");
    let frames = (managed_end - base) / PAGE_SIZE;

    ALLOCATOR.with(|state| {
        assert!(!state.initialized, "physical allocator initialized twice");
        state.base = base;
        state.frames = frames;
        // All frames begin permanently reserved. Release only full pages after
        // the linker-aligned kernel end, then apply firmware reservations.
        let first_free = (kernel_end - base) / PAGE_SIZE;
        for index in first_free..frames {
            let mask = !(1u64 << (index % 64));
            state.used[index / 64] &= mask;
            state.permanent[index / 64] &= mask;
            state.free += 1;
        }
        state.reserve(Region {
            start: device_tree as u64,
            end: device_tree.checked_add(tree.size()).expect("FDT address overflow") as u64,
        });
        tree.reserved_regions(|region| state.reserve(region))
            .unwrap_or_else(|e| panic!("physical memory reservations: {}", e));
        state.initialized = true;
        state.stats()
    })
}

/// Returns the physical address of a free 4 KiB frame.
pub fn allocate_frame() -> Option<usize> {
    ALLOCATOR.with(|state| {
        assert!(state.initialized, "physical allocator used before init");
        if state.free == 0 { return None; }
        for word_index in 0..((state.frames + 63) / 64) {
            let available = !state.used[word_index];
            if available == 0 { continue; }
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
        if !state.initialized { return Err("physical allocator used before init"); }
        if address < state.base || address >= state.base + state.frames * PAGE_SIZE || address % PAGE_SIZE != 0 {
            return Err("frame outside managed RAM or unaligned");
        }
        let index = (address - state.base) / PAGE_SIZE;
        let bit = 1u64 << (index % 64);
        if state.permanent[index / 64] & bit != 0 { return Err("frame is reserved"); }
        let word = &mut state.used[index / 64];
        if *word & bit == 0 { return Err("frame is already free"); }
        *word &= !bit;
        state.free += 1;
        Ok(())
    })
}
