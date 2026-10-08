//! x86-64 early architecture support. Firmware services end before entry.

pub mod console;
pub mod trap;

use crate::boot_info::BootInfo;

pub fn init(boot_info: &BootInfo) {
    console::init(&boot_info.framebuffer);
    trap::init();
}
