//! Wolfram Memory System
//! VMO-everything. No anonymous memory. No implicit backing. Ever.

pub mod bitmap;
pub mod vmo;

pub fn init(device_tree: usize) -> bitmap::MemoryStats {
    bitmap::init(device_tree)
}
