//! Wolfram Capability System
//!
//! This is the core of the kernel. Everything else exists to serve this.
//!
//! A capability is an unforgeable token that simultaneously names a resource
//! and authorizes operations on it. If you hold the handle, you have the
//! authority the handle encodes. There is no separate check.
//!
//! If you're reading this, you're either debugging something horrible
//! or you're curious. Either way, welcome.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Rights: u32 {
        /// Can copy this handle to give to another process.
        const DUPLICATE = 1 << 0;
        /// Can move this handle into another process.
        const TRANSFER  = 1 << 1;
        /// Can read from this object.
        const READ      = 1 << 2;
        /// Can write to this object.
        const WRITE     = 1 << 3;
        /// Can map this VMO as executable.
        const EXECUTE   = 1 << 4;
        /// Can map this VMO into an address space.
        const MAP       = 1 << 5;
        /// Can raise signals on this object.
        const SIGNAL    = 1 << 6;
        /// Can wait for signals on this object.
        const WAIT      = 1 << 7;
        /// Can read metadata without using the object.
        /// Separates "can use" from "can see". Intentional.
        const INSPECT   = 1 << 8;
        /// Can perform lifecycle operations (kill, destroy).
        const MANAGE    = 1 << 9;
    }
}

/// A typed, rights-bearing kernel object handle.
///
/// T: the kernel object type (VMO, Channel, Process, ...)
/// R: the rights bitmask, encoded at the type level via PhantomData
///
/// Zero runtime cost for the type information.
/// The kernel enforces at runtime.
/// The type system enforces at compile time.
/// You get both for the price of one.
pub struct Handle<T, R> {
    raw: u32,
    _type: PhantomData<T>,
    _rights: PhantomData<R>,
}

/// Capability node — the indirection layer that makes revocation instant.
///
/// Processes hold handles to CapNodes, not to objects directly.
/// To revoke: invalidate the node. Every handle through it dies immediately.
/// One pointer indirection. Negligible overhead. Worth every cycle.
pub struct CapNode {
    id: u32,
    valid: AtomicBool,
    rights: Rights,
}

impl CapNode {
    pub fn new(id: u32, rights: Rights) -> Self {
        Self {
            id,
            valid: AtomicBool::new(true),
            rights,
        }
    }

    /// Revoke this capability node.
    /// All handles pointing through it are immediately dead.
    /// No further action needed. That's the point of the indirection.
    pub fn revoke(&self) {
        self.valid.store(false, Ordering::Release);
    }

    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    pub fn rights(&self) -> Rights {
        self.rights
    }
}

/// Per-process handle table.
/// The only thing connecting a process to the rest of the system.
/// Born empty. Populated at spawn time. No implicit inheritance.
/// There is no ambient access. There is no "but I'm root."
/// There is only: do you have the handle or not.
pub struct HandleTable {
    entries: alloc::vec::Vec<Option<HandleEntry>>,
}

struct HandleEntry {
    node: alloc::sync::Arc<CapNode>,
    rights: Rights,
}

impl HandleTable {
    pub fn new() -> Self {
        // born empty
        // that's the whole model right here
        Self {
            entries: alloc::vec::Vec::new(),
        }
    }

    pub fn lookup(&self, handle: u32) -> Option<(alloc::sync::Arc<CapNode>, Rights)> {
        let entry = self.entries.get(handle as usize)?.as_ref()?;
        if !entry.node.is_valid() {
            // revoked. the node is dead. the handle is dead.
            // this is working as intended.
            return None;
        }
        Some((entry.node.clone(), entry.rights))
    }

    pub fn insert(&mut self, node: alloc::sync::Arc<CapNode>, rights: Rights) -> u32 {
        for (i, slot) in self.entries.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(HandleEntry { node, rights });
                return i as u32;
            }
        }
        self.entries.push(Some(HandleEntry { node, rights }));
        (self.entries.len() - 1) as u32
    }

    pub fn remove(&mut self, handle: u32) -> Option<(alloc::sync::Arc<CapNode>, Rights)> {
        let entry = self.entries.get_mut(handle as usize)?.take()?;
        Some((entry.node, entry.rights))
    }

    pub fn count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }
}

extern crate alloc;
