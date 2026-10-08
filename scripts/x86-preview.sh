#!/usr/bin/env bash
# Build the freestanding x86 kernel and check the loader-facing ELF shape.
# This does not boot the image.
set -euo pipefail

kernel_repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$kernel_repo_root/wolfram"

target=x86_64-unknown-none
if ! rustup target list --installed --toolchain nightly | rg -qx "$target"; then
    echo "install the bare-metal target first: rustup target add $target --toolchain nightly" >&2
    exit 1
fi
cargo build --target "$target" "$@"

profile=debug
if [[ " $* " == *" --release "* ]]; then
    profile=release
fi
target_dir=${CARGO_TARGET_DIR:-target}
image="$target_dir/$target/$profile/wolfram"

readelf -h "$image" | rg -q 'Type:.*DYN'
readelf -h "$image" | rg -q 'Machine:.*X86-64'
if readelf -d "$image" | rg -q '\(NEEDED\)' || readelf -l "$image" | rg -q 'INTERP'; then
    echo "x86 kernel unexpectedly needs a dynamic library or interpreter" >&2
    exit 1
fi
if readelf -r "$image" | awk '/R_X86_64_/ && $3 != "R_X86_64_RELATIVE" { bad = 1; print } END { exit !bad }'; then
    echo "x86 kernel has a relocation unsupported by the UEFI loader" >&2
    exit 1
fi

echo "Validated ET_DYN kernel: $image"
