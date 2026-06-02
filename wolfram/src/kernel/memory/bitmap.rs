//! Bitmap physical memory allocator.
//! One bit per 4KB page frame. Boot-time allocator.
//! Phase 2: replaced by buddy allocator.
//! Simple enough to trust before we have anything else working.
//!
//! If you're reading this after Phase 2 shipped, something went wrong.

pub fn init() {
    // Phase 1: stub — no real RAM detection yet
    // just enough to satisfy the init call
}
