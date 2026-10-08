# Wolfram UEFI loader

This standalone Rust `no_std` UEFI application loads `WOLFRAM.ELF` from the
same FAT filesystem as the loader. The kernel must be an x86-64 ELF64
position-independent `ET_DYN` image with only `R_X86_64_RELATIVE` dynamic
relocations and a SysV entry point accepting `BootInfo *` in `RDI`. The loader
asks UEFI for a contiguous run of pages, loads and relocates the image there,
then exits boot services and jumps to the ELF entry on a dedicated stack.

Build with the pinned nightly toolchain after installing both x86 targets:

```sh
rustup target add x86_64-unknown-none x86_64-unknown-uefi --toolchain nightly
scripts/x86-preview.sh --release
(cd boot/uefi && cargo +nightly build --target x86_64-unknown-uefi --release)
```

Stage files into a FAT32 partition or a QEMU EFI system partition directory:

```sh
boot/uefi/package.sh /path/to/wolfram-kernel-elf /path/to/fat-root
```

The result contains `EFI/BOOT/BOOTX64.EFI` and `WOLFRAM.ELF`. The image is
unsigned, so firmware Secure Boot policy must permit it. The loader requires a
GOP mode with RGB or BGR 32-bit pixels, and passes its framebuffer and the UEFI
memory map in the versioned `BootInfo` structure. Only
`EfiConventionalMemory` is passed as usable. All loader allocations and boot
data are `EfiLoaderCode`/`EfiLoaderData` and remain reserved in the handoff.

Before the jump, the loader installs four-level identity page tables covering
firmware-described ranges, the kernel, its stack and boot data, and the GOP
framebuffer. Kernel segments receive per-page writable and executable
permissions. The kernel must validate the handoff and must never call UEFI
boot services afterward.

The release kernel and loader have reached the expected `spawn init` panic in
QEMU/OVMF. The debug kernel has passed the same boot check. The MSI
B650M-A PRO WIFI has also booted the loader and kernel from USB, initialized
the physical allocator, and reached the expected diagnosed panic.

Run `make x86-smoke` from the repository root to repeat the debug and release
boot checks and an opt-in invalid-opcode exception probe. The smoke script
builds its kernels in a temporary target directory, so its deliberate fault
image cannot replace the ordinary kernel intended for a USB stick.

For an optical-media QEMU test, install `libisoburn`, `dosfstools`, and
`mtools` (Arch package names), then run `make x86-iso`. This creates an ignored
`wolfram.iso` at the repository root. It embeds a FAT El Torito image with both
the UEFI loader and `WOLFRAM.ELF`; placing the ELF only in the ISO filesystem
would not work because the loader reads from its own FAT filesystem. Boot it
with QEMU/OVMF using `-cdrom wolfram.iso -boot order=d`. The ISO tests the CD
boot path; the MSI USB test still uses a FAT32 partition with the two files.
