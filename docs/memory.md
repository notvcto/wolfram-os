# Wolfram Memory Model

Wolfram is a "VMO-everything" system.

## Virtual Memory Objects (VMOs)
Every region of virtual memory is backed by a named kernel object called a VMO.
- **No Anonymous Memory:** There is no implicit backing. If you allocate memory, you are creating or mapping a VMO.
- **Hardware Enforcement:** Rights (READ, WRITE, EXECUTE) are enforced at the page table level by the MMU. This means capability rights translate directly into hardware protection.

## Physical Allocation
The physical memory allocator operates in two phases:
1. **Boot:** A simple bitmap allocator (Phase 1) manages initial setup.
2. **Runtime:** A buddy allocator (Phase 2) takes over for dynamic runtime allocation, supporting page splitting and coalescing.

*See [docs/architecture.md](architecture.md) for architectural context.*
