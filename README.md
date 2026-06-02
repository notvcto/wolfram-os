# Wolfram

> A capability-based microkernel. Zero ambient authority. Programs earn every resource they touch.

Wolfram is a microkernel operating system written in Rust, targeting RISC-V 64-bit hardware first.
It is not a Linux distribution. It is not a fork. It is a kernel built from first principles around
one idea: **programs should not have access to anything they weren't explicitly given.**

There is no root. There is no `sudo`. There is no ambient authority.
Every resource is a capability handle. Every handle has explicit rights.
Every transfer is intentional. Every revocation is instant.

This is not a weekend project. It is not finished. It may never be finished.
But the architecture is right, and that matters more than being finished.

---

## The Model

In most operating systems, security is a layer on top of the kernel.
Permission bits, UIDs, ACLs, SELinux policies — all of these are answers to the question
"should this process be allowed to do this?" asked after the process already named the resource.

Wolfram asks a different question: **how did this process get a reference to that resource at all?**

If it doesn't have a handle, it can't name it. If it can't name it, the question never arises.
Capability possession *is* the permission. There is no separate check.

This is called capability-based security. It is not a new idea.
Wolfram is a new implementation of it, in Rust, for modern hardware,
with zero legacy constraints and zero tolerance for ambient authority.

---

## Architecture

```
Wolfram Kernel (microkernel)
├── Capability enforcement     — the core. handle tables, rights, attenuation, revocation.
├── Scheduler                  — jobs → processes → threads. no fork. no ambient inheritance.
├── IPC                        — async channels + FastCall. handle transfer is move semantics.
├── Memory                     — VMO-everything. no anonymous memory. hardware-enforced rights.
└── HAL                        — architecture abstraction. RISC-V first. ARM64 later.

Ferrum (userspace foundation)
├── libc port                  — musl-based, Wolfram syscall ABI
├── VFS server                 — filesystem abstraction as a capability service
├── Driver framework           — userspace drivers, MMIO via Physical VMO
└── Init                       — job tree root. everything descends from here.

fer (unified system CLI)
├── fer pkg                    — package management
├── fer drv                    — driver management
├── fer cap                    — capability inspection and revocation
├── fer jobs                   — job tree visualization
├── fer mem                    — memory and VMO inspection
└── fer ipc                    — IPC channel monitoring
```

---

## Capability Model

Every kernel object — process, thread, VMO, channel, device — has handles.
Handles are unforgeable. You cannot guess one. You cannot forge one.
The kernel is the only entity that creates them.

Every handle carries a rights bitmask:

```
DUPLICATE   TRANSFER   READ   WRITE   EXECUTE
MAP         SIGNAL     WAIT   INSPECT MANAGE
```

When you pass a handle to another process, you can only pass equal or fewer rights.
Authority flows downward. It can never be amplified.
A child cannot have more access than its parent granted.
A parent cannot grant more than it holds.

Revocation is instant. Capability nodes are invalidated at the kernel level.
Every handle pointing through a revoked node loses access immediately.

In Rust, this enforced at compile time too:

```rust
// these are different types — you cannot mix them up
Handle<Vmo, Read>
Handle<Vmo, ReadWrite>
Handle<Channel, ReadWrite>

// this is a compile error, not a runtime error
fn write(h: Handle<Vmo, ReadWrite>, data: &[u8]) { ... }
write(read_only_handle, data) // ← compiler rejects this
```

Two layers of enforcement. The compiler catches the obvious mistakes.
The kernel catches everything else.

---

## What Wolfram Is Not

- Not a Linux distribution
- Not a Linux replacement (yet)
- Not API-compatible with POSIX (by design)
- Not finished
- Not safe to run in production
- Not something you should daily drive today

---

## What Wolfram Will Be

- A kernel you can understand completely
- A security model that isn't bolted on
- A system where `fer cap audit <pid>` tells you everything
- Something worth daily driving
- A community project with a real identity

---

## Versioning

Wolfram uses element names for releases.

| Tier | Elements | Meaning |
|---|---|---|
| Nightly | Uranium, Thorium, Plutonium | Radioactive. Unstable by definition. |
| Unstable | Lithium, Sodium, Potassium | Reactive. Handle with care. |
| RC | Fluorine, Chlorine, Bromine | Getting closer. Still sharp edges. |
| Stable | Helium, Neon, Argon, Krypton, Xenon | Inert. Doesn't react. Ships. |

Current: **Uranium-238** (nightly — first boots, nothing works yet, that's fine)

---

## Roadmap

**Phase 1 — Boots (now)**
- [ ] Bootloader handoff via RISC-V SBI
- [ ] Serial output
- [ ] Physical memory detection
- [ ] Bitmap allocator
- [ ] Basic trap handling
- [ ] Doesn't triple fault

**Phase 2 — Kernel Core**
- [ ] Capability system
- [ ] Virtual memory + VMOs
- [ ] Buddy allocator
- [ ] Process/thread model (no fork)
- [ ] Job tree
- [ ] Async channels
- [ ] FastCall IPC

**Phase 3 — Userspace**
- [ ] Init process
- [ ] Basic shell
- [ ] `fer` CLI skeleton
- [ ] musl libc port
- [ ] VFS server

**Phase 4 — Drivers**
- [ ] Driver framework
- [ ] NIC driver (virtio in QEMU)
- [ ] Block device driver
- [ ] Basic filesystem (read-only initramfs)

**Phase 5 — `fer` Complete**
- [ ] `fer pkg` — package management
- [ ] `fer drv` — driver management
- [ ] `fer cap` — capability inspection
- [ ] `fer mem` — memory inspection
- [ ] `fer ipc` — IPC monitoring

**Daily driver milestone: somewhere past Phase 5.**
It will take years. That's fine.

---

## Building

Requirements:
- Rust nightly (we use features that aren't stable yet — appropriate for a kernel)
- RISC-V target: `rustup target add riscv64gc-unknown-none-elf`
- QEMU: `qemu-system-riscv64`
- A tolerance for triple faults

```bash
git clone https://github.com/notvcto/wolfram-os
cd wolfram
make run     # boots in QEMU
make debug   # boots with GDB server on :1234
```

Right now `make run` boots, prints something to serial, and probably panics.
That's expected. That's Phase 1.

---

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) first.
Read [docs/architecture.md](docs/architecture.md) second.
Read [docs/capability-model.md](docs/capability-model.md) third.

Then find something in the roadmap that isn't checked off and start there.

The kernel is written in Rust. The docs are written in English.
Both should be precise, honest, and free of unnecessary complexity.

If you find a security issue in the capability model specifically, that's important —
open an issue marked `[SECURITY]` and be detailed.

---

## License

GPL v2. Same as Linux. Derivatives stay open.

---

*Wolfram. Tungsten melts at 3422°C.*
*W —*
