//! Architecture abstraction layer.
//! Add new targets here. The kernel core stays untouched.

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;
