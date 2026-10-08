#!/usr/bin/env bash
# Build a UEFI optical ISO with a FAT El Torito boot image.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
if [[ $# -gt 1 ]]; then
    echo "usage: $0 [OUTPUT_ISO]" >&2
    exit 2
fi
output_iso=${1:-"$repo_root/wolfram.iso"}

for command in xorriso mkfs.vfat mmd mcopy truncate; do
    if ! command -v "$command" >/dev/null; then
        echo "missing $command (Arch packages: libisoburn dosfstools mtools)" >&2
        exit 1
    fi
done

"$repo_root/scripts/x86-preview.sh" --release
(cd "$repo_root/boot/uefi" && cargo +nightly build --release --target x86_64-unknown-uefi)

iso_work_dir=$(mktemp -d /tmp/wolfram-iso.XXXXXX)
trap 'rm -rf -- "$iso_work_dir"' EXIT
efi_image="$iso_work_dir/iso/EFI/BOOT/efiboot.img"
mkdir -p "$(dirname "$efi_image")"
truncate -s 64M "$efi_image"
mkfs.vfat -F 32 -n WOLFRAM "$efi_image" >/dev/null
mmd -i "$efi_image" ::/EFI ::/EFI/BOOT
mcopy -i "$efi_image" \
    "$repo_root/boot/uefi/target/x86_64-unknown-uefi/release/wolfram-uefi-loader.efi" \
    ::/EFI/BOOT/BOOTX64.EFI
mcopy -i "$efi_image" \
    "$repo_root/wolfram/target/x86_64-unknown-none/release/wolfram" \
    ::/WOLFRAM.ELF

xorriso -as mkisofs -R -J -V WOLFRAM \
    -e EFI/BOOT/efiboot.img -no-emul-boot \
    -o "$output_iso" "$iso_work_dir/iso"
echo "Built UEFI optical ISO: $output_iso"
