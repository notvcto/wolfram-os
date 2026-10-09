//! Wolfram Capability System
//!
//! This is the core of the kernel. Everything else exists to serve this.
//!
//! If you're reading this, you're either debugging something horrible
//! or you're curious. Either way, welcome.

#![allow(dead_code)]

pub mod allocator;
pub mod handle_table;

pub use handle_table::{HandleTable, CapError};
pub use allocator::{CapNodeAllocator, CAP_NODE_ALLOCATOR};

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};
use core::ptr::NonNull;

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

/// Marker trait for kernel-managed objects that can be held via handles.
pub trait KernelObject: Send + Sync {
    /// Human-readable type name for diagnostics and `fer cap audit`.
    fn type_name(&self) -> &'static str;
}

/// Runtime type tag for kernel objects. Serialisable, matchable in syscall dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectType {
    Process  = 0,
    Thread   = 1,
    Vmo      = 2,
    Channel  = 3,
    Job      = 4,
    Resource = 5,
}

// Placeholders for kernel objects
pub struct Process;
impl KernelObject for Process { fn type_name(&self) -> &'static str { "Process" } }

pub struct Thread;
impl KernelObject for Thread { fn type_name(&self) -> &'static str { "Thread" } }

pub struct Vmo;
impl KernelObject for Vmo { fn type_name(&self) -> &'static str { "Vmo" } }

pub struct Channel;
impl KernelObject for Channel { fn type_name(&self) -> &'static str { "Channel" } }

pub struct Job;
impl KernelObject for Job { fn type_name(&self) -> &'static str { "Job" } }

pub struct Resource;
impl KernelObject for Resource { fn type_name(&self) -> &'static str { "Resource" } }

/// Marker traits for compile-time rights enforcement.
/// Each right type is an uninhabited ZST.
pub struct Read;
pub struct Write;
pub struct ReadWrite;
pub struct Execute;

/// A typed, rights-annotated handle to a kernel object.
///
/// `T` is the kernel object type (must impl `KernelObject`).
/// `R` is a rights marker type for compile-time enforcement.
///
/// The handle also carries a CapNode pointer for runtime enforcement
/// at the syscall boundary.
pub struct Handle<T: KernelObject, R> {
    /// Index into the owning process's handle table.
    raw: u32,
    /// Pointer to the capability node backing this handle.
    node: NonNull<CapNode>,
    _type: PhantomData<T>,
    _rights: PhantomData<R>,
}

/// A capability node — the indirection layer between handles and kernel objects.
///
/// Handle → CapNode → Object.
/// Revoking a CapNode kills every handle that points through it.
pub struct CapNode {
    /// Unique identifier within the kernel's global node table.
    id: u32,
    /// Revocation flag. Atomic so revocation is wait-free.
    valid: AtomicBool,
    /// Maximum rights this node grants. Attenuated children
    /// have a subset of these.
    rights: Rights,
    /// Pointer to the underlying kernel object.
    /// Typed as `*const ()` + `ObjectType` rather than a trait object
    /// to avoid a vtable indirection on the hot path.
    object: NonNull<()>,
    object_type: ObjectType,
    /// Parent node in the derivation tree. `None` for root capabilities
    /// created by the kernel at boot.
    parent: Option<NonNull<CapNode>>,
}

impl CapNode {
    pub fn new(
        id: u32,
        rights: Rights,
        object: NonNull<()>,
        object_type: ObjectType,
        parent: Option<NonNull<CapNode>>,
    ) -> Self {
        Self { 
            id, 
            valid: AtomicBool::new(true), 
            rights,
            object,
            object_type,
            parent,
        }
    }

    /// Revoke this node. Every handle through it dies immediately.
    pub fn revoke(&self) {
        self.valid.store(false, Ordering::Release);
    }

    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    pub fn rights(&self) -> Rights { self.rights }

    pub fn object_type(&self) -> ObjectType { self.object_type }
    pub fn parent(&self) -> Option<NonNull<CapNode>> { self.parent }

    /// Get a typed reference to the underlying object.
    /// # Safety
    /// Caller must guarantee `T` matches `self.object_type`.
    pub unsafe fn object<T>(&self) -> &T {
        unsafe { self.object.cast::<T>().as_ref() }
    }

    /// Create a child capability node with attenuated rights.
    ///
    /// The child's rights must be a subset of this node's rights.
    /// Returns `Err(CapError::InsufficientRights)` if `new_rights`
    /// contains rights not present in `self.rights`.
    pub fn attenuate(
        &self,
        new_rights: Rights,
        allocator: &mut CapNodeAllocator,
    ) -> Result<NonNull<CapNode>, CapError> {
        if !self.rights.contains(new_rights) {
            return Err(CapError::InsufficientRights);
        }
        let id = allocator.next_id();
        let child = allocator.alloc(CapNode::new(
            id,
            new_rights,
            self.object,
            self.object_type,
            Some(NonNull::from(self)),
        )).map_err(|_| CapError::TableFull)?;
        Ok(child)
    }
}

pub fn init() {
    // Initialize the global capability node allocator.
    // The allocator pool is backed by pages from the bitmap allocator.
    unsafe { CAP_NODE_ALLOCATOR.init() };
}
