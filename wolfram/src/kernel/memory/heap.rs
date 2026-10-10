//! Buddy Allocator for the Wolfram Kernel Heap.
//!
//! Provides dynamic memory for kernel objects (Capabilities, VMO structs, etc).
//! Max buddy size is 4 KiB (1 page). Allocations > 4 KiB are forwarded directly
//! to the physical bitmap allocator.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::kernel::memory::bitmap;

const PAGE_SIZE: usize = 4096;
const MIN_BLOCK_SIZE: usize = 32;
const MAX_ORDER: usize = 7; // 4096 = 32 * 2^7

struct FreeBlock {
    next: Option<*mut FreeBlock>,
}

struct BuddyState {
    free_lists: [Option<*mut FreeBlock>; MAX_ORDER + 1],
}

pub struct BuddyAllocator {
    state: core::cell::UnsafeCell<BuddyState>,
    lock: AtomicBool,
}

unsafe impl Sync for BuddyAllocator {}

#[global_allocator]
static HEAP_ALLOCATOR: BuddyAllocator = BuddyAllocator {
    state: core::cell::UnsafeCell::new(BuddyState {
        free_lists: [None; MAX_ORDER + 1],
    }),
    lock: AtomicBool::new(false),
};

impl BuddyAllocator {
    fn with<R>(&self, f: impl FnOnce(&mut BuddyState) -> R) -> R {
        while self.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        let result = f(unsafe { &mut *self.state.get() });
        self.lock.store(false, Ordering::Release);
        result
    }
}

fn size_to_order(size: usize) -> usize {
    let size = size.next_power_of_two().max(MIN_BLOCK_SIZE);
    size.trailing_zeros() as usize - MIN_BLOCK_SIZE.trailing_zeros() as usize
}

fn order_to_size(order: usize) -> usize {
    MIN_BLOCK_SIZE << order
}

unsafe impl GlobalAlloc for BuddyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());
        if size == 0 {
            return ptr::null_mut();
        }

        if size > PAGE_SIZE {
            if layout.align() > PAGE_SIZE {
                // The physical bitmap allocator only guarantees PAGE_SIZE alignment.
                // For now, fail allocations requiring larger alignments.
                return ptr::null_mut();
            }
            let count = size.div_ceil(PAGE_SIZE);
            return bitmap::allocate_frames(count).map(|addr| addr as *mut u8).unwrap_or(ptr::null_mut());
        }

        let order = size_to_order(size);

        self.with(|state| {
            for o in order..=MAX_ORDER {
                if let Some(block) = state.free_lists[o] {
                    state.free_lists[o] = unsafe { (*block).next };
                    
                    // Split down to the required order
                    let mut current_order = o;
                    let current_block = block as usize;
                    
                    while current_order > order {
                        current_order -= 1;
                        let half_size = order_to_size(current_order);
                        let buddy = (current_block + half_size) as *mut FreeBlock;
                        
                        unsafe {
                            (*buddy).next = state.free_lists[current_order];
                        }
                        state.free_lists[current_order] = Some(buddy);
                    }
                    
                    return current_block as *mut u8;
                }
            }
            
            // Need a new page
            if let Some(frame) = bitmap::allocate_frame() {
                let mut current_order = MAX_ORDER;
                let current_block = frame;
                
                while current_order > order {
                    current_order -= 1;
                    let half_size = order_to_size(current_order);
                    let buddy = (current_block + half_size) as *mut FreeBlock;
                    
                    unsafe {
                        (*buddy).next = state.free_lists[current_order];
                    }
                    state.free_lists[current_order] = Some(buddy);
                }
                
                return current_block as *mut u8;
            }
            
            ptr::null_mut()
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let size = layout.size().max(layout.align());
        if size > PAGE_SIZE {
            let count = size.div_ceil(PAGE_SIZE);
            bitmap::free_frames(ptr as usize, count).expect("invalid free");
            return;
        }

        let order = size_to_order(size);
        
        self.with(|state| {
            let mut current_block = ptr as usize;
            let mut current_order = order;
            
            while current_order < MAX_ORDER {
                let buddy = current_block ^ order_to_size(current_order);
                
                // Search for buddy in current free list
                let mut prev: Option<*mut FreeBlock> = None;
                let mut curr = state.free_lists[current_order];
                let mut found = false;
                
                while let Some(node) = curr {
                    if node as usize == buddy {
                        // Remove buddy from list
                        if let Some(p) = prev {
                            unsafe { (*p).next = (*node).next };
                        } else {
                            state.free_lists[current_order] = unsafe { (*node).next };
                        }
                        found = true;
                        break;
                    }
                    prev = Some(node);
                    curr = unsafe { (*node).next };
                }
                
                if found {
                    // Merge blocks
                    current_block = core::cmp::min(current_block, buddy);
                    current_order += 1;
                } else {
                    break;
                }
            }
            
            if current_order == MAX_ORDER {
                // Free the entire page back to the physical allocator
                bitmap::free_frame(current_block).expect("invalid page free");
            } else {
                // Add to the appropriate free list
                let block = current_block as *mut FreeBlock;
                unsafe {
                    (*block).next = state.free_lists[current_order];
                }
                state.free_lists[current_order] = Some(block);
            }
        })
    }
}
