# Uranium-238 v0.1.2

This release delivers the Buddy Allocator, the first major structural requirement of Phase 2 (Kernel Core). It provides the dynamic heap memory required by the capability system, VMOs, and the process tree.

## Included

- **Dynamic Kernel Heap:** Implemented a thread-safe Buddy Allocator (`wolfram/src/kernel/memory/heap.rs`) as the `#[global_allocator]` for the kernel, supporting block sizes from 32 bytes up to 4 KiB.
- **Physical Memory Integration:** Integrated with the Phase 1 physical frame bitmap allocator, dynamically requesting new pages when order free lists are exhausted.
- **Contiguous Frame Support:** Enhanced the bitmap allocator to support contiguous multi-page allocations (`allocate_frames(count)`) to back large kernel structures.
- **Large Allocation Bypassing:** Requests larger than a single page (4 KiB) automatically bypass the buddy system and are serviced directly by the physical frame allocator.
- **Safety Hardening:** Added explicit layout alignment constraints for large allocations, ensuring strict adherence to the Rust `GlobalAlloc` contract by gracefully returning null if alignment exceeds 4 KiB.
- **Rigorous Verification:** Added comprehensive in-kernel smoke tests validating buddy block splitting, coalescing to `MAX_ORDER`, large allocation bounds, and explicit OOM handling.

This release establishes the memory foundation necessary to begin implementing Virtual Memory Objects (VMOs) and the Job tree.
