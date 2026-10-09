use core::ptr::NonNull;
use super::CapNode;

const POOL_SIZE: usize = 1024;

pub struct CapNodeAllocator {
    nodes: [Option<CapNode>; POOL_SIZE],
    next_id: u32,
}

impl CapNodeAllocator {
    pub const fn new() -> Self {
        const INIT_NODE: Option<CapNode> = None;
        Self {
            nodes: [INIT_NODE; POOL_SIZE],
            next_id: 1,
        }
    }

    pub unsafe fn init(&mut self) {
        // Any specific init if we were dynamically allocating memory, 
        // but since we are statically allocated we just zero/reset
    }

    pub fn alloc(&mut self, node: CapNode) -> Result<NonNull<CapNode>, ()> {
        for (_i, slot) in self.nodes.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(node);
                // In a real buddy/slab allocator we'd use raw pointers and init.
                // For this phase 2 array implementation, we just take the reference.
                let ptr = NonNull::from(slot.as_ref().unwrap());
                return Ok(ptr);
            }
        }
        Err(())
    }

    pub fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

pub static mut CAP_NODE_ALLOCATOR: CapNodeAllocator = CapNodeAllocator::new();
