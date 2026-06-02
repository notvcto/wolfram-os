//! RISC-V serial output via SBI legacy console putchar.
//! This is how we talk to the outside world before we have anything else.
//! Without this you are debugging by staring at QEMU's reset counter.

pub fn write_byte(byte: u8) {
    sbi_call(1, byte as usize, 0, 0);
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

fn sbi_call(eid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") eid,
        );
    }
    ret
}

pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    Writer.write_fmt(args).ok();
}

struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::arch::riscv64::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! kprintln {
    () => ($crate::kprint!("\n"));
    ($fmt:literal $(, $($arg:tt)*)?) => {
        $crate::kprint!(concat!($fmt, "\n") $(, $($arg)*)?)
    };
}
