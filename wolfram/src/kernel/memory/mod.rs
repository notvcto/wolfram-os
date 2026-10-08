//! Wolfram Memory System
//! VMO-everything. No anonymous memory. No implicit backing. Ever.

pub mod bitmap;
pub mod vmo;

use crate::boot_info::{BootInfo, MemoryRegion};

pub fn init_from_boot_info(info: &BootInfo) -> bitmap::MemoryStats {
    bitmap::init_from_boot_info(info)
}

/// Translate the OpenSBI device tree into the same boot contract used by UEFI.
#[cfg(target_arch = "riscv64")]
pub fn init(device_tree: usize) -> bitmap::MemoryStats {
    use crate::arch::device_tree::{DeviceTree, Region};

    const MAX_BOOT_REGIONS: usize = 128;

    // SAFETY: OpenSBI passes an identity-mapped FDT in a1.
    let tree = unsafe { DeviceTree::from_ptr(device_tree) }
        .unwrap_or_else(|e| panic!("physical memory detection: {}", e));
    extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }
    let kernel_start = &raw const __kernel_start as u64;
    let kernel_end = &raw const __kernel_end as u64;
    let mut regions = [MemoryRegion::EMPTY; MAX_BOOT_REGIONS];
    let mut len = 0;
    let mut push = |region: MemoryRegion| {
        assert!(len < MAX_BOOT_REGIONS, "too many FDT memory regions");
        regions[len] = region;
        len += 1;
    };

    let mut kernel_bank_start = None;
    tree.memory_regions(|Region { start, end }| {
        if start <= kernel_start && end >= kernel_end {
            kernel_bank_start = Some(start);
        }
        push(MemoryRegion::usable(start, end));
    })
    .unwrap_or_else(|e| panic!("physical memory detection: {}", e));
    let bank_start = kernel_bank_start.expect("no FDT RAM bank contains the kernel");

    // OpenSBI and its runtime data occupy the RAM before the linked kernel.
    // The FDT memory nodes include this area, so reserve it explicitly.
    push(MemoryRegion::reserved(bank_start, kernel_end));
    push(MemoryRegion::reserved(
        device_tree as u64,
        (device_tree as u64)
            .checked_add(tree.size() as u64)
            .expect("FDT address overflow"),
    ));
    tree.reserved_regions(|Region { start, end }| {
        push(MemoryRegion::reserved(start, end));
    })
    .unwrap_or_else(|e| panic!("physical memory reservations: {}", e));

    let info = BootInfo::new(&regions[..len], kernel_start, kernel_end);
    bitmap::init_from_boot_info(&info)
}
