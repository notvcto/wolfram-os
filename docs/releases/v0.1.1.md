# Phase 2 Capability Foundation (v0.1.1)

This release ships the structural foundation for the Wolfram Phase 2 capability system, resolving architectural discrepancies and setting up the type-safe enforcement layer.

## Included

- **Type-Safe Capabilities:** Overhauled `capabilities/mod.rs` with `ObjectType`, the `KernelObject` trait, and type-safe `Handle<T: KernelObject, R: RightMarker>` to enforce capability permutations at compile time.
- **Node & Allocator:** Implemented `CapNode` with `NonNull<()>` object references and a working `CapNodeAllocator` that initializes cleanly at boot.
- **Syscall Boundaries:** Added the x86-64 `SyscallFrame` and wired up the x86-64 syscall dispatch stub.
- **IPC & Memory Documentation:** Created `docs/ipc.md` and `docs/memory.md` detailing the intended Phase 2 architecture for buddy allocation and MMU enforcement.
- **Cleanup:** Scrubbed the legacy `W —` signoff pattern from panic handlers and boot messages, officially closing out the Phase 1 narrative.

This release does not yet implement process management or a usable shell; it provides the internal kernel invariants required before VMOs or Tasks can be allocated.
