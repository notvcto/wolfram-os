#!/usr/bin/env bash
# Build a freestanding x86 kernel with the installed host core as a preview.
# This checks the loader-facing ELF shape; it does not boot the image.
set -euo pipefail

kernel_repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$kernel_repo_root/wolfram"

export RUSTFLAGS='-C linker=rust-lld -C relocation-model=pic -C code-model=small -C no-redzone=yes -C link-arg=--pie -C link-arg=--no-dynamic-linker -C link-arg=-Tsrc/arch/x86_64/linker.ld'
cargo build --target x86_64-unknown-linux-gnu "$@"

profile=debug
if [[ " $* " == *" --release "* ]]; then
    profile=release
fi
image="target/x86_64-unknown-linux-gnu/$profile/wolfram"

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
