#!/bin/sh
# Boot the RISC-V kernel with two RAM sizes and multiple harts. The kernel's
# Phase 1 panic spins forever, so timeout's exit status 124 is expected.
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
(cd "$repo_dir/wolfram" && cargo build --quiet)
kernel="$repo_dir/wolfram/target/riscv64gc-unknown-none-elf/debug/wolfram"
log=$(mktemp)
trap 'rm -f "$log"' EXIT HUP INT TERM

for size in 128 256; do
    status=0
    timeout 8s qemu-system-riscv64 \
        -machine virt -smp 4 -nographic -bios default \
        -kernel "$kernel" -m "${size}M" >"$log" 2>&1 || status=$?

    if [ "$status" -ne 124 ]; then
        cat "$log"
        echo "QEMU exited unexpectedly for ${size} MiB (status $status)" >&2
        exit 1
    fi

    case "$size" in
        128) ram_end=0x88000000 ;;
        256) ram_end=0x90000000 ;;
    esac
    if ! grep -Eq "\\[mem\\]   RAM: 0x80000000\\.\\.${ram_end}, [1-9][0-9]* free pages" "$log" \
        || ! grep -Fq '[mem]   physical allocator: ok' "$log" \
        || ! grep -Fq 'where:       src/main.rs:' "$log" \
        || ! grep -Fq 'it begins.' "$log"; then
        cat "$log"
        echo "RISC-V smoke output missing for ${size} MiB" >&2
        exit 1
    fi
    echo "RISC-V smoke passed: ${size} MiB, 4 harts, expected Phase 1 panic"
done
