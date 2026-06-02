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
mod kernel;

use core::panic::PanicInfo;

core::arch::global_asm!(include_str!("arch/riscv64/boot.S"));

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    arch::init();

    kprintln!("W — good morning. probably.");
    kprintln!();
    kprintln!("Wolfram/0.1.0 (Uranium-238)");
    kprintln!("capability-based microkernel — RISC-V 64");
    kprintln!();

    kernel::memory::init();
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
