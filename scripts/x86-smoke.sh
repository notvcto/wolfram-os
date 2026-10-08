#!/usr/bin/env bash
# Exercise the UEFI handoff, normal Phase 1 panic, and x86 exception path.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ovmf_code=${OVMF_CODE:-/usr/share/edk2/x64/OVMF_CODE.4m.fd}
ovmf_vars=${OVMF_VARS:-/usr/share/edk2/x64/OVMF_VARS.4m.fd}

for required in "$ovmf_code" "$ovmf_vars"; do
    if [[ ! -f "$required" ]]; then
        echo "missing OVMF firmware: $required" >&2
        exit 1
    fi
done
for command in qemu-system-x86_64 timeout rg; do
    if ! command -v "$command" >/dev/null; then
        echo "missing command: $command" >&2
        exit 1
    fi
done

trial_dir=$(mktemp -d /tmp/wolfram-x86-smoke.XXXXXX)
smoke_passed=0
cleanup() {
    if [[ "$smoke_passed" -eq 1 ]]; then
        rm -rf -- "$trial_dir"
    else
        echo "smoke logs retained in $trial_dir" >&2
    fi
}
trap cleanup EXIT

(cd "$repo_root/boot/uefi" && cargo +nightly build --release --target x86_64-unknown-uefi)

run_case() {
    local profile=$1 scenario=$2 ram=$3
    local cargo_args=(--features qemu-debug-port)
    local log="$trial_dir/$profile-$scenario.log"
    local qemu_log="$trial_dir/$profile-$scenario-qemu.log"
    if [[ "$profile" == release ]]; then
        cargo_args+=(--release)
    fi
    if [[ "$scenario" == exception ]]; then
        cargo_args=(--features x86-exception-probe --release)
    fi

    "$repo_root/scripts/x86-preview.sh" "${cargo_args[@]}"
    "$repo_root/boot/uefi/package.sh" \
        "$repo_root/wolfram/target/x86_64-unknown-none/$profile/wolfram" \
        "$trial_dir/esp"
    cp -- "$ovmf_vars" "$trial_dir/OVMF_VARS.fd"

    local status=0
    timeout 8s qemu-system-x86_64 \
        -machine q35 -m "${ram}M" -smp 1 \
        -drive "if=pflash,format=raw,unit=0,readonly=on,file=$ovmf_code" \
        -drive "if=pflash,format=raw,unit=1,file=$trial_dir/OVMF_VARS.fd" \
        -drive "if=virtio,format=raw,readonly=on,file=fat:$trial_dir/esp" \
        -vga std -display none -monitor none -serial none -no-reboot \
        -debugcon "file:$log" -global isa-debugcon.iobase=0xe9 \
        >"$qemu_log" 2>&1 || status=$?

    if [[ "$status" -ne 124 ]]; then
        cat "$qemu_log" >&2
        echo "QEMU exited unexpectedly ($profile $scenario, status $status)" >&2
        exit 1
    fi
    if ! rg -Fq '[mem]   physical allocator: ok' "$log" \
        || ! rg -Fq 'Phase 1 boot checks complete.' "$log" \
        || ! rg -Fq 'W O L F R A M   K E R N E L   P A N I C' "$log"; then
        cat "$log" >&2
        echo "missing kernel boot diagnostics ($profile $scenario)" >&2
        exit 1
    fi
    if [[ "$scenario" == exception ]]; then
        if ! rg -Fq 'probing x86 invalid-opcode exception...' "$log" \
            || ! rg -Fq 'detail: x86 exception vector=6 error=0x0 rip=' "$log" \
            || ! rg -Fq 'where:       src/arch/x86_64/trap.rs:' "$log"; then
            cat "$log" >&2
            echo "missing x86 exception diagnostics" >&2
            exit 1
        fi
    elif ! rg -Fq 'detail: not yet implemented: spawn init' "$log"; then
        cat "$log" >&2
        echo "missing expected spawn-init panic" >&2
        exit 1
    fi
    echo "x86 QEMU smoke passed: $profile $scenario, ${ram} MiB"
}

run_case debug normal 512
run_case release normal 1024
run_case release exception 512
smoke_passed=1
