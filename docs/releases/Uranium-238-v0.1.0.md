# Uranium-238 v0.1.0

Wolfram's first nightly release marks completion of the Phase 1 boot milestone.
It demonstrates a shared kernel core starting on both RISC-V and x86-64, including
a successful boot on the MSI B650M-A PRO WIFI.

## Included

- RISC-V 64 boot through OpenSBI, with device-tree RAM detection, a bitmap frame
  allocator, trap handling, and secondary-hart parking.
- x86-64 UEFI boot with firmware memory-map handoff, framebuffer diagnostics,
  exception reporting, and a boot allocator probe.
- Repeatable QEMU smoke checks for both architectures.
- A UEFI optical ISO containing the loader and kernel.
- MSI B650M-A PRO WIFI USB boot through memory allocation to the expected,
  diagnosed `spawn init` panic.

## Trying the ISO

Use the attached `wolfram.iso` with a UEFI virtual machine such as QEMU/OVMF.
For real hardware, the Phase 1 check used a FAT32 USB stick with
`EFI/BOOT/BOOTX64.EFI` and `WOLFRAM.ELF`. Secure Boot may need to be disabled
for the unsigned loader. Ventoy boot has not been verified.

This release is an early kernel boot milestone. It intentionally stops at the
`spawn init` panic. Capability enforcement, userspace isolation, process
management, and a usable shell are not implemented yet.
