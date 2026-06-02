//! Wolfram Syscall Interface
//! Every syscall takes a handle as its first argument.
//! There is no open-by-path. There is no get-by-name.
//! You have a handle or you don't.
//! Phase 2 implementation.

use crate::arch::riscv64::trap::TrapFrame;

#[allow(dead_code)]
pub fn dispatch(_nr: usize, _frame: &mut TrapFrame) {
    // Phase 2
}
