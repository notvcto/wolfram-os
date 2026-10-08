KERNEL := wolfram/target/riscv64gc-unknown-none-elf/debug/wolfram

.PHONY: all build run debug clean

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

clean:
	cd wolfram && cargo clean
