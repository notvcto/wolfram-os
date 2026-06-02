//! RISC-V trap handler — exceptions, interrupts, syscalls.

use core::arch::global_asm;

global_asm!(include_str!("trap.S"));

#[repr(C)]
pub struct TrapFrame {
    pub ra: usize,  pub sp: usize,  pub gp: usize,  pub tp: usize,
    pub t0: usize,  pub t1: usize,  pub t2: usize,
    pub s0: usize,  pub s1: usize,
    pub a0: usize,  pub a1: usize,  pub a2: usize,  pub a3: usize,
    pub a4: usize,  pub a5: usize,  pub a6: usize,  pub a7: usize,
    pub s2: usize,  pub s3: usize,  pub s4: usize,  pub s5: usize,
    pub s6: usize,  pub s7: usize,  pub s8: usize,  pub s9: usize,
    pub s10: usize, pub s11: usize,
    pub t3: usize,  pub t4: usize,  pub t5: usize,  pub t6: usize,
    pub sepc: usize,
}

#[no_mangle]
extern "C" fn kernel_trap_handler(frame: &mut TrapFrame) {
    let scause: usize;
    let stval: usize;
    unsafe {
        core::arch::asm!("csrr {}, scause", out(reg) scause);
        core::arch::asm!("csrr {}, stval",  out(reg) stval);
    }

    let is_interrupt = (scause >> 63) != 0;
    let code = scause & !(1 << 63);

    if is_interrupt {
        match code {
            1 => {}  // supervisor software interrupt
            5 => {}  // supervisor timer
            9 => {}  // supervisor external
            _ => panic!("unknown interrupt: {}", code),
        }
    } else {
        match code {
            8 | 9 => {
                // ecall — syscall dispatch (Phase 2)
                frame.sepc += 4;
            }
            _ => {
                panic!("unhandled exception: cause={} tval={:#x} sepc={:#x}",
                       code, stval, frame.sepc);
            }
        }
    }
}

pub fn init() {
    extern "C" { fn kernel_trap_entry(); }
    unsafe {
        core::arch::asm!(
            "csrw stvec, {}",
            in(reg) kernel_trap_entry as *const () as usize,
        );
    }
}
