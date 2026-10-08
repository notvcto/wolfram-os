# Wolfram UEFI loader

This standalone Rust `no_std` UEFI application loads `WOLFRAM.ELF` from the
same FAT filesystem as the loader. The kernel must be an x86-64 ELF64
position-independent `ET_DYN` image with only `R_X86_64_RELATIVE` dynamic
relocations and a SysV entry point accepting `BootInfo *` in `RDI`. The loader
asks UEFI for a contiguous run of pages, loads and relocates the image there,
then exits boot services and jumps to the ELF entry on a dedicated stack.

Build with the pinned nightly toolchain after installing its UEFI target:

```sh
rustup target add x86_64-unknown-uefi --toolchain nightly
cd boot/uefi
cargo +nightly build --target x86_64-unknown-uefi --release
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
