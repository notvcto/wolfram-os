//! Wolfram Kernel — entry point.
//!
//! This is where the kernel starts. Everything before this
//! was the bootloader's problem. Everything after this is ours.
//!
//! Phase 1 goal: boot, print something, don't triple fault.
//! Current status: working on it.

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

mod arch;
mod boot_info;
mod kernel;

// The installed host std target can build a freestanding preview when the
// x86-64 bare-metal target is unavailable. Its prebuilt core expects libc
// memory routines, so supply them locally for that preview only.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod host_shims;

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
pub unsafe extern "C" fn kernel_main(boot_info: *const boot_info::BootInfo) -> ! {
    let info = boot_info.as_ref().expect("missing boot information");
    arch::init(info);
    kernel_start("x86-64", || kernel::memory::init_from_boot_info(info))
}

fn kernel_start(
    board: &str,
    init_memory: impl FnOnce() -> kernel::memory::bitmap::MemoryStats,
) -> ! {
    kprintln!("W — good morning. probably.");
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

    kernel::capabilities::init();
    kprintln!("[cap]   capability system: ok");

    kprintln!();
    kprintln!("kernel initialized.");
    kprintln!("spawning init...");
    kprintln!();

    todo!("spawn init — Phase 3")
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::panic::kernel_panic(info)
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("oom: failed to allocate {} bytes", layout.size())
}
