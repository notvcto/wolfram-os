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
#![feature(naked_functions)]

extern crate alloc;

mod arch;
mod kernel;

use core::panic::PanicInfo;

/// Kernel entry point. Called by the bootloader after basic setup.
/// At this point: we're in supervisor mode, MMU is off, stack exists.
/// Everything else: our problem.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // initialize architecture-specific hardware
    arch::init();

    kprintln!("Wolfram/{} (Uranium-238)", env!("CARGO_PKG_VERSION"));
    kprintln!("capability-based microkernel — RISC-V 64");
    kprintln!();

    // initialize kernel subsystems in dependency order
    kernel::memory::init();
    kprintln!("[mem]   physical allocator: ok");

    kernel::capabilities::init();
    kprintln!("[cap]   capability system: ok");

    kernel::scheduler::init();
    kprintln!("[sched] scheduler: ok");

    kernel::ipc::init();
    kprintln!("[ipc]   channel system: ok");

    kprintln!();
    kprintln!("kernel initialized. spawning init.");
    kprintln!("(init does not exist yet. this will panic.)");
    kprintln!("(that's expected. that's Phase 1.)");
    kprintln!();

    // TODO: load init process with root capabilities
    // TODO: hand off to scheduler
    // TODO: never return
    todo!("spawn init — Phase 3")
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::panic::kernel_panic(info)
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("oom: failed to allocate {} bytes (align {})",
           layout.size(), layout.align())
}
