//! RISC-V 64-bit architecture layer.
//! The kernel core never imports from here directly.
//! Everything goes through the HAL boundary.

pub mod serial;
pub mod trap;
pub mod device_tree;

pub fn init() {
    trap::init();
}
