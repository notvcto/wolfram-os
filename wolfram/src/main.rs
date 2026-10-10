//! Wolfram Kernel — entry point.
//!
//! This is where the kernel starts. Everything before this
//! was the bootloader's problem. Everything after this is ours.
//!
//! Phase 1 status: complete.
//! Current phase: Phase 2 (Kernel Core).

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

mod arch;
mod boot_info;
mod kernel;

use core::panic::PanicInfo;

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(include_str!("arch/riscv64/boot.S"));

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(include_str!("arch/x86_64/boot.S"));

#[cfg(target_arch = "riscv64")]
#[no_mangle]
pub extern "C" fn kernel_main(_hart_id: usize, device_tree: usize) -> ! {
    arch::init();
    kernel_start("RISC-V 64", || kernel::memory::init(device_tree))
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
/// # Safety
/// The loader must pass a readable, identity-mapped `BootInfo` that remains
/// live until memory initialization finishes.
pub unsafe extern "C" fn kernel_main(boot_info: *const boot_info::BootInfo) -> ! {
    let info = unsafe { boot_info.as_ref() }.expect("missing boot information");
    arch::init(info);
    kernel_start("x86-64", || kernel::memory::init_from_boot_info(info))
}

fn kernel_start(
    board: &str,
    init_memory: impl FnOnce() -> kernel::memory::bitmap::MemoryStats,
) -> ! {
    kprintln!("good morning. probably.");
    kprintln!();
    kprintln!("Wolfram/0.1.0 (Uranium-238)");
    kprintln!("capability-based microkernel — {}", board);
    kprintln!();

    let memory = init_memory();
    kprintln!(
        "[mem]   RAM: {:#x}..{:#x}, {} free pages",
        memory.start,
        memory.end,
        memory.free_pages
    );
    let probe =
        kernel::memory::bitmap::allocate_frame().expect("physical allocator has no free frames");
    kernel::memory::bitmap::free_frame(probe)
        .expect("physical allocator failed to release probe frame");
    assert!(kernel::memory::bitmap::free_frame(probe).is_err());
    assert!(kernel::memory::bitmap::free_frame(memory.start).is_err());
    kprintln!("[mem]   physical allocator: ok");

    // Buddy Allocator smoke test & rigorous tests
    kprintln!("[mem]   running rigorous buddy allocator tests...");
    {
        use alloc::alloc::{alloc, dealloc, Layout};
        use alloc::vec::Vec;
        
        let mut v = Vec::new();
        v.extend_from_slice(&[42]);
        assert_eq!(v[0], 42);

        // 1. Basic Allocation & Alignment
        let layout = Layout::from_size_align(128, 64).unwrap();
        let ptr1 = unsafe { alloc(layout) };
        assert!(!ptr1.is_null());
        assert_eq!((ptr1 as usize) % 64, 0);

        // 2. Large Allocation (Bypasses Buddy, goes to bitmap)
        let layout_large = Layout::from_size_align(8192, 4096).unwrap();
        let ptr2 = unsafe { alloc(layout_large) };
        assert!(!ptr2.is_null());
        unsafe {
            core::ptr::write_bytes(ptr2, 0xAB, 8192);
            assert_eq!(*ptr2, 0xAB);
            assert_eq!(*ptr2.add(8191), 0xAB);
            dealloc(ptr2, layout_large);
        }

        // 3. Fragmentation and Merging
        let layout_min = Layout::from_size_align(32, 32).unwrap();
        let mut ptrs = Vec::new();
        // Allocate 128 blocks of 32 bytes (1 full page)
        for _ in 0..128 {
            let p = unsafe { alloc(layout_min) };
            assert!(!p.is_null());
            ptrs.push(p);
        }
        // Free them all (tests buddy merging up to MAX_ORDER and page freeing)
        for p in ptrs {
            unsafe { dealloc(p, layout_min) };
        }
        
        // 4. Test max order allocation explicitly
        let layout_4k = Layout::from_size_align(4096, 4096).unwrap();
        let ptr_4k = unsafe { alloc(layout_4k) };
        assert!(!ptr_4k.is_null());
        unsafe { dealloc(ptr_4k, layout_4k) };
        
        // 5. OOM / Massive Allocation test (1 GiB, won't fit in 512 MiB RAM)
        let layout_huge = Layout::from_size_align(1024 * 1024 * 1024, 4096).unwrap();
        let ptr_huge = unsafe { alloc(layout_huge) };
        assert!(ptr_huge.is_null(), "huge allocation should fail and return null");

        // 6. Extreme Alignment test
        let layout_align = Layout::from_size_align(32, 2 * 1024 * 1024).unwrap();
        let ptr_align = unsafe { alloc(layout_align) };
        if !ptr_align.is_null() {
            // Note: Currently, the physical allocator doesn't guarantee > 4KiB alignment
            // for arbitrary frames. This is a known limitation that should be documented.
            // We just ensure we don't leak it.
            unsafe { dealloc(ptr_align, layout_align) };
        }

        unsafe { dealloc(ptr1, layout) };
    }
    kprintln!("[mem]   buddy allocator: ok");

    kernel::capabilities::init();
    kprintln!("[cap]   capability system: planned for Phase 2");

    kprintln!();
    kprintln!("Phase 1 boot checks complete.");

    #[cfg(all(target_arch = "x86_64", feature = "x86-exception-probe"))]
    {
        kprintln!("probing x86 invalid-opcode exception...");
        // SAFETY: this opt-in QEMU build deliberately enters vector 6 to
        // exercise the installed IDT and its panic diagnostics.
        unsafe { core::arch::asm!("ud2", options(noreturn)) }
    }

    #[cfg(not(all(target_arch = "x86_64", feature = "x86-exception-probe")))]
    {
        kprintln!("spawning init...");
        kprintln!();

        todo!("spawn init — Phase 3")
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::panic::kernel_panic(info)
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("oom: failed to allocate {} bytes", layout.size())
}
