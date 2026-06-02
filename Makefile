KERNEL := wolfram/target/riscv64gc-unknown-none-elf/debug/wolfram

.PHONY: all run debug clean

all: $(KERNEL)

$(KERNEL):
	cd wolfram && cargo build

run: $(KERNEL)
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(KERNEL) \
		-m 128M

debug: $(KERNEL)
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(KERNEL) \
		-m 128M \
		-s -S

clean:
	cd wolfram && cargo clean
