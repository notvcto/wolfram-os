//! Early x86-64 exception diagnostics. Interrupts remain disabled in Phase 1.

use core::arch::{asm, global_asm};

global_asm!(include_str!("trap.S"));

#[repr(C, packed)]
struct TablePointer {
    limit: u16,
    base: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const EMPTY: Self = Self {
        offset_low: 0,
        selector: 0,
        ist: 0,
        flags: 0,
        offset_mid: 0,
        offset_high: 0,
        reserved: 0,
    };

    fn new(address: usize) -> Self {
        Self {
            offset_low: address as u16,
            selector: 0x08,
            ist: 0,
            flags: 0x8e, // present, ring 0, interrupt gate
            offset_mid: (address >> 16) as u16,
            offset_high: (address >> 32) as u32,
            reserved: 0,
        }
    }
}

// Long-mode code and data selectors. A TSS is added with userspace later.
static GDT: [u64; 3] = [0, 0x00af_9a00_0000_ffff, 0x00cf_9200_0000_ffff];
static mut IDT: [IdtEntry; 256] = [IdtEntry::EMPTY; 256];

extern "C" {
    static x86_trap_vectors: [usize; 32];
    fn x86_unexpected_interrupt();
}

pub fn init() {
    let gdt = TablePointer {
        limit: (core::mem::size_of_val(&GDT) - 1) as u16,
        base: GDT.as_ptr() as u64,
    };
    // SAFETY: one bootstrap processor executes this while interrupts are off.
    unsafe {
        asm!("lgdt [{}]", in(reg) &gdt, options(readonly, nostack));
        asm!(
            "push 0x08",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            out("rax") _,
        );

        let idt = &raw mut IDT;
        let default = IdtEntry::new(x86_unexpected_interrupt as *const () as usize);
        for vector in 0..256 {
            (*idt)[vector] = default;
        }
        for vector in 0..32 {
            (*idt)[vector] = IdtEntry::new(x86_trap_vectors[vector]);
        }
        let descriptor = TablePointer {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: idt as u64,
        };
        asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack));
    }
}

#[repr(C)]
struct TrapFrame {
    vector: u64,
    error: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
}

#[no_mangle]
extern "C" fn x86_trap_handler(frame: *const TrapFrame) -> ! {
    // SAFETY: assembly passes a pointer to the hardware frame on this stack.
    let frame = unsafe { &*frame };
    panic!(
        "x86 exception vector={} error={:#x} rip={:#x} cs={:#x} rflags={:#x}",
        frame.vector, frame.error, frame.rip, frame.cs, frame.rflags
    );
}
