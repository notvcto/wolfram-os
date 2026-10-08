# Phase 1 Boot Plan

Wolfram needs two boot paths into one kernel core. RISC-V 64 under QEMU/OpenSBI
is the working reference. x86-64 UEFI is the next hardware target because the
available test machine is an MSI B650M-A PRO WIFI. The first x86-64 milestone is
to boot from a USB stick, print diagnostics on that machine, and reach the same
intentional `spawn init` panic seen on RISC-V. This is an early kernel boot, not
an interactive or secure userspace system.

The [README roadmap](../README.md#roadmap) tracks completion. This document
defines the order of work and what counts as evidence for each step.

## 1. Preserve the RISC-V baseline

- Keep `cargo build` from `wolfram/` and the existing QEMU boot working while
  adding another target. A build from the repository root does not pick up
  `wolfram/.cargo/config.toml` automatically.
- Capture the detected RAM range, free frame count, and expected panic in a
  repeatable QEMU smoke check. Exercise at least two RAM sizes.
- Fix and exercise the RISC-V trap path before calling its boot reliable. Give
  each active hart a separate stack or explicitly park secondary harts.

The current bitmap allocator manages the RAM bank containing the kernel, up to
4 GiB. It is a boot allocator, not the Phase 2 buddy allocator.

## 2. Define a common handoff to the kernel

Create a small, versioned boot-information structure containing usable and
reserved physical memory ranges, the kernel image range, and an optional
framebuffer description. Keep early logging architecture-specific. Include
firmware table information only where the kernel needs it.
The architecture layer supplies this structure; kernel memory initialization
must not depend on whether the source was a RISC-V device tree or a UEFI memory
map. Copy or reserve every piece of boot data the kernel will retain.

Keep firmware calls and hardware setup behind the architecture boundary in
`arch/`. The kernel core owns frame allocation and, later, capabilities,
processes, VMOs, and IPC. Build both targets after each interface change so the
second port keeps this boundary honest.

**Done when:** both boot paths can pass validated memory ranges to the same
allocator interface, and the RISC-V QEMU boot still reaches its expected panic.

## 3. Boot x86-64 in QEMU with UEFI

- Add an x86-64 kernel target and a separate UEFI loader. The loader obtains
  the UEFI memory map and GOP framebuffer, allocates and loads the kernel, then
  passes owned boot information to it. Do not assume the RISC-V kernel's fixed
  physical load address is free on a PC.
- Take the final memory map and call `ExitBootServices` before entering the
  kernel. The kernel must never call UEFI boot services afterward or allocate
  frames marked reserved by firmware.
- Establish a kernel stack, page tables that map the kernel and framebuffer,
  and basic GDT/IDT exception handling.
  Start on the bootstrap processor; leave other processors inactive until
  their stacks and interrupt setup exist. Keep interrupts controlled during
  this initial boot.
- Print through the GOP framebuffer so diagnostics will work on the MSI board
  without a serial port. A QEMU serial backend can provide extra diagnostics.
  Report the memory range and frame count, exercise one frame allocation and
  release, then reach the intentional `spawn init` panic.

**Done when:** a reproducible QEMU/OVMF run reaches that panic in debug and
release builds; invalid or reserved memory is not handed to the allocator;
an unexpected exception produces a useful diagnostic instead of a silent reset.

The QEMU/OVMF runs now reach the expected panic in debug and release builds.
`make x86-smoke` repeats both boots and triggers a deliberate invalid-opcode
exception in an isolated build; that path reports vector 6, error code, and
instruction pointer on the framebuffer and QEMU debug port. The MSI board is
still untested.

## 4. Boot from USB on the MSI board

Package the loader as a UEFI removable-media application (`EFI/BOOT/BOOTX64.EFI`)
with its kernel image on a FAT32 USB partition. Use the board's UEFI boot menu;
the firmware may require Secure Boot to be configured for the image. This step
does not require Wolfram to contain a USB driver: UEFI reads the files before
the kernel takes control.

Record the board firmware version and exact boot result. Compare the memory map
and reserved ranges with the QEMU path. Fix assumptions exposed by the board
without putting board-specific policy in the kernel core.

**Done when:** repeated cold boots on the MSI board display Wolfram's memory
diagnostics and the expected panic without a silent reset or memory fault.
This is the Phase 1 real-hardware milestone, not a daily-driver milestone.

## 5. Continue the shared kernel roadmap

Only after the boot paths are stable should Phase 2 claim a capability system
or userspace isolation. The current capability initialization is a stub and
the boot message says so. Handle rights and revocation must be enforced in the
shared core. Each architecture must also enforce user/kernel separation and VMO
rights in its page tables, including non-executable writable memory. A boot
screen alone does not establish those security properties.

ACPI, interrupt controllers, PCIe, USB, storage, graphics drivers, and
multi-processor scheduling can be added as their later milestones need them.
The UEFI loader can read the USB stick for Phase 1; using that stick after boot
requires drivers and explicit capabilities. Hardware-assisted DMA isolation
also needs separate work before userspace drivers can safely control devices.

Keep the Uranium-238 release tag pending until the Phase 1 roadmap is complete
and the relevant boot paths pass their checks.
