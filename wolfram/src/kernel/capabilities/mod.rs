//! Wolfram Capability System
//!
//! This is the core of the kernel. Everything else exists to serve this.
//!
//! If you're reading this, you're either debugging something horrible
//! or you're curious. Either way, welcome.

#![allow(dead_code)]

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Rights: u32 {
        const DUPLICATE = 1 << 0;
        const TRANSFER  = 1 << 1;
        const READ      = 1 << 2;
        const WRITE     = 1 << 3;
        const EXECUTE   = 1 << 4;
        const MAP       = 1 << 5;
        const SIGNAL    = 1 << 6;
        const WAIT      = 1 << 7;
        const INSPECT   = 1 << 8;
        const MANAGE    = 1 << 9;
    }
}

pub struct Handle<T, R> {
    raw: u32,
    _type: PhantomData<T>,
    _rights: PhantomData<R>,
}

pub struct CapNode {
    id: u32,
    valid: AtomicBool,
    rights: Rights,
}

impl CapNode {
    pub fn new(id: u32, rights: Rights) -> Self {
        Self { id, valid: AtomicBool::new(true), rights }
    }

    /// Revoke this node. Every handle through it dies immediately.
    pub fn revoke(&self) {
        self.valid.store(false, Ordering::Release);
    }

    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    pub fn rights(&self) -> Rights { self.rights }
}

pub fn init() {
    // capability system initialized
    // handle tables are per-process, created at spawn time
    // nothing to do globally until we have processes
}
