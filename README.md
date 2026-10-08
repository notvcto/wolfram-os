# Wolfram

> A capability-based microkernel. Zero ambient authority. Programs earn every resource they touch.

Wolfram is a microkernel operating system written in Rust. RISC-V 64 is its
current QEMU reference port; x86-64 UEFI is the next physical-hardware target.
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
└── HAL                        — architecture abstraction. RISC-V and x86-64 first.

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

Planned first release: **Uranium-238** (nightly). No release tag yet; Phase 1
boot checks must pass first.

---

## Roadmap

**Phase 1 — Reliable boots (now)**

RISC-V 64 QEMU reference:
- [x] OpenSBI handoff and early serial output in QEMU
- [x] Device-tree physical memory detection and bitmap allocator
- [x] Exercise trap handling and park secondary harts before the boot stack
- [x] Repeatable QEMU smoke check at 128 and 256 MiB with four harts

x86-64 UEFI PC target:
- [ ] Common boot information and architecture boundary
- [ ] UEFI loader and framebuffer diagnostics in QEMU/OVMF
- [ ] Firmware memory map, reserved ranges, and boot allocator
- [ ] Basic exception handling and expected panic in QEMU/OVMF
- [ ] USB boot to the same diagnostics on the MSI B650M-A PRO WIFI

See the [Phase 1 boot plan](docs/boot-plan.md) for sequence and acceptance
criteria. Phase 1 is complete when both QEMU ports and the MSI test machine
reach a diagnosed, intentional panic without a silent reset.

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

## Building the RISC-V reference port

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

Right now `make run` boots in QEMU, prints to serial, and reaches the expected
`spawn init` panic. The x86-64 UEFI build and USB boot path are planned in
[docs/boot-plan.md](docs/boot-plan.md).

The boot allocator reads RAM from the firmware device tree and manages the bank
containing the kernel (up to 4 GiB). It keeps the kernel image, device tree, and
firmware-declared reserved ranges unavailable for allocation. Init is planned
for Phase 3.

The x86-64 kernel and UEFI loader are in progress. `scripts/x86-preview.sh`
checks the kernel ELF that the loader expects; see the
[UEFI loader notes](boot/uefi/README.md) for its build and packaging steps.
Neither QEMU/OVMF nor the MSI board has booted this path yet.

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
