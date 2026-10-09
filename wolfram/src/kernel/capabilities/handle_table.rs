use core::ptr::NonNull;
use super::{CapNode, CapError};

const MAX_HANDLES: usize = 256; // Phase 2 limit. Generous for early testing.

/// Per-process handle table. Maps `u32` indices to `CapNode` pointers.
///
/// Processes are born with an empty table. Capabilities are explicitly
/// passed in at spawn time. No ambient authority.
pub struct HandleTable {
    /// Sparse array of capability nodes. `None` = slot is free.
    /// Phase 2 uses a fixed-size array. Phase 3 can grow dynamically.
    entries: [Option<NonNull<CapNode>>; MAX_HANDLES],
    /// Number of live entries (for diagnostics and `fer cap audit`).
    count: u32,
}

impl HandleTable {
    pub fn new() -> Self {
        Self {
            entries: [None; MAX_HANDLES],
            count: 0,
        }
    }

    /// Insert a CapNode, return the assigned handle index.
    pub fn insert(&mut self, node: NonNull<CapNode>) -> Result<u32, CapError> {
        if self.count as usize >= MAX_HANDLES {
            return Err(CapError::TableFull);
        }

        for (i, entry) in self.entries.iter_mut().enumerate() {
            if entry.is_none() {
                *entry = Some(node);
                self.count += 1;
                return Ok(i as u32);
            }
        }
        
        Err(CapError::TableFull)
    }

    /// Look up a handle. Returns the CapNode if the handle is valid and the
    /// node hasn't been revoked.
    pub fn lookup(&self, handle: u32) -> Result<&CapNode, CapError> {
        let entry = self.entries.get(handle as usize).copied().flatten().ok_or(CapError::InvalidHandle)?;
        
        let node = unsafe { entry.as_ref() };
        if !node.is_valid() {
            return Err(CapError::Revoked);
        }
        
        Ok(node)
    }

    /// Remove a handle (used on close or transfer-out).
    pub fn remove(&mut self, handle: u32) -> Result<NonNull<CapNode>, CapError> {
        if let Some(entry) = self.entries.get_mut(handle as usize) {
            if let Some(node) = entry.take() {
                self.count -= 1;
                return Ok(node);
            }
        }
        Err(CapError::InvalidHandle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapError {
    InvalidHandle,
    Revoked,
    TableFull,
    InsufficientRights,
}
