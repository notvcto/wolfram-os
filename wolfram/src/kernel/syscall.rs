//! Wolfram Syscall Interface
//! Every syscall takes a handle as its first argument.
//! There is no open-by-path. There is no get-by-name.
//! You have a handle or you don't.
//! Phase 2 implementation.

#[cfg(target_arch = "riscv64")]
use crate::arch::riscv64::trap::TrapFrame;

#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
pub fn dispatch(_nr: usize, _frame: &mut TrapFrame) {
    // Phase 2
}

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::syscall::SyscallFrame;

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn dispatch(_nr: usize, _frame: &mut SyscallFrame) {
    // Phase 2
}
