//! Wolfram Memory System
//! VMO-everything. No anonymous memory. No implicit backing. Ever.

pub mod vmo;
pub mod bitmap;

pub fn init(device_tree: usize) -> bitmap::MemoryStats {
    bitmap::init(device_tree)
}
