KERNEL := wolfram/target/riscv64gc-unknown-none-elf/debug/wolfram

.PHONY: all build run debug x86-preview x86-smoke x86-iso clean

all: build

build:
	cd wolfram && cargo build

run: build
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(KERNEL) \
		-m 128M

debug: build
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(KERNEL) \
		-m 128M \
		-s -S

x86-preview:
	scripts/x86-preview.sh

x86-smoke:
	scripts/x86-smoke.sh

x86-iso:
	scripts/x86-iso.sh

clean:
	cd wolfram && cargo clean
