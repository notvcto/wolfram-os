//! Wolfram Memory System
//! VMO-everything. No anonymous memory. No implicit backing. Ever.

pub mod vmo;
pub mod bitmap;

pub fn init() {
    bitmap::init();
}
