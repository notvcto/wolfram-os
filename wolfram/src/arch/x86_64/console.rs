//! Early GOP framebuffer console. It deliberately needs no heap or UEFI calls.

use core::cell::UnsafeCell;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::boot_info::{Framebuffer, PIXEL_FORMAT_BGR, PIXEL_FORMAT_RGB};

const SCALE: usize = 2;
const CELL_WIDTH: usize = 6 * SCALE;
const CELL_HEIGHT: usize = 8 * SCALE;

struct Console {
    base: usize,
    size: usize,
    width: usize,
    height: usize,
    stride: usize,
    x: usize,
    y: usize,
}

impl Console {
    const fn empty() -> Self {
        Self { base: 0, size: 0, width: 0, height: 0, stride: 0, x: 0, y: 0 }
    }

    fn clear(&mut self) {
        if self.base == 0 { return; }
        for offset in (0..self.size).step_by(4) {
            // SAFETY: init checked that the framebuffer is mapped and sized.
            unsafe { ((self.base + offset) as *mut u32).write_volatile(0); }
        }
        self.x = 0;
        self.y = 0;
    }

    fn put_pixel(&self, x: usize, y: usize) {
        if x >= self.width || y >= self.height { return; }
        let offset = (y * self.stride + x) * 4;
        if offset + 4 <= self.size {
            // White is the same byte sequence in RGB and BGR GOP modes.
            unsafe { ((self.base + offset) as *mut u32).write_volatile(0x00ff_ffff); }
        }
    }

    fn newline(&mut self) {
        self.x = 0;
        self.y += CELL_HEIGHT;
        if self.y + CELL_HEIGHT > self.height { self.clear(); }
    }

    fn put_char(&mut self, ch: char) {
        if self.base == 0 { return; }
        if ch == '\n' { self.newline(); return; }
        if ch == '\r' { self.x = 0; return; }
        if self.x + CELL_WIDTH > self.width { self.newline(); }
        let glyph = glyph(match ch {
            '—' | '–' | '─' => '-',
            '═' => '=',
            c if c.is_ascii() => c.to_ascii_uppercase(),
            _ => '?',
        });
        for (column, bits) in glyph.iter().enumerate() {
            for row in 0..7 {
                if bits & (1 << row) != 0 {
                    for sy in 0..SCALE {
                        for sx in 0..SCALE {
                            self.put_pixel(self.x + column * SCALE + sx, self.y + row * SCALE + sy);
                        }
                    }
                }
            }
        }
        self.x += CELL_WIDTH;
    }
}

struct LockedConsole {
    lock: AtomicBool,
    state: UnsafeCell<Console>,
}

// The spin lock serializes output if another hart is added later.
unsafe impl Sync for LockedConsole {}

static CONSOLE: LockedConsole = LockedConsole {
    lock: AtomicBool::new(false),
    state: UnsafeCell::new(Console::empty()),
};

impl LockedConsole {
    fn with<R>(&self, f: impl FnOnce(&mut Console) -> R) -> R {
        while self.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        struct Unlock<'a>(&'a AtomicBool);
        impl Drop for Unlock<'_> {
            fn drop(&mut self) { self.0.store(false, Ordering::Release); }
        }
        let _unlock = Unlock(&self.lock);
        // SAFETY: the lock is held for the duration of the closure.
        f(unsafe { &mut *self.state.get() })
    }
}

pub fn init(fb: &Framebuffer) {
    let Ok(base) = usize::try_from(fb.base) else { return; };
    let Ok(size) = usize::try_from(fb.size) else { return; };
    let width = fb.width as usize;
    let height = fb.height as usize;
    let stride = fb.stride as usize;
    let Some(visible_bytes) = stride.checked_mul(height).and_then(|n| n.checked_mul(4)) else {
        return;
    };
    if base == 0 || base & 3 != 0 || width < CELL_WIDTH || height < CELL_HEIGHT
        || stride < width || !matches!(fb.pixel_format, PIXEL_FORMAT_RGB | PIXEL_FORMAT_BGR)
        || visible_bytes > size || base.checked_add(visible_bytes).is_none()
    { return; }
    CONSOLE.with(|console| {
        *console = Console { base, size: visible_bytes, width, height, stride, x: 0, y: 0 };
        console.clear();
    });
}

pub fn _print(args: fmt::Arguments) {
    struct Writer<'a>(&'a mut Console);
    impl Write for Writer<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for ch in s.chars() { self.0.put_char(ch); }
            Ok(())
        }
    }
    CONSOLE.with(|console| { let _ = Writer(console).write_fmt(args); });
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::arch::x86_64::console::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! kprintln {
    () => ($crate::kprint!("\n"));
    ($fmt:literal $(, $($arg:tt)*)?) => {
        $crate::kprint!(concat!($fmt, "\n") $(, $($arg)*)?)
    };
}

// Five columns, seven rows, least significant bit at the top.
fn glyph(ch: char) -> [u8; 5] {
    match ch {
        'A' => [0x7e,0x11,0x11,0x11,0x7e], 'B' => [0x7f,0x49,0x49,0x49,0x36],
        'C' => [0x3e,0x41,0x41,0x41,0x22], 'D' => [0x7f,0x41,0x41,0x22,0x1c],
        'E' => [0x7f,0x49,0x49,0x49,0x41], 'F' => [0x7f,0x09,0x09,0x09,0x01],
        'G' => [0x3e,0x41,0x49,0x49,0x7a], 'H' => [0x7f,0x08,0x08,0x08,0x7f],
        'I' => [0x00,0x41,0x7f,0x41,0x00], 'J' => [0x20,0x40,0x41,0x3f,0x01],
        'K' => [0x7f,0x08,0x14,0x22,0x41], 'L' => [0x7f,0x40,0x40,0x40,0x40],
        'M' => [0x7f,0x02,0x0c,0x02,0x7f], 'N' => [0x7f,0x04,0x08,0x10,0x7f],
        'O' => [0x3e,0x41,0x41,0x41,0x3e], 'P' => [0x7f,0x09,0x09,0x09,0x06],
        'Q' => [0x3e,0x41,0x51,0x21,0x5e], 'R' => [0x7f,0x09,0x19,0x29,0x46],
        'S' => [0x46,0x49,0x49,0x49,0x31], 'T' => [0x01,0x01,0x7f,0x01,0x01],
        'U' => [0x3f,0x40,0x40,0x40,0x3f], 'V' => [0x1f,0x20,0x40,0x20,0x1f],
        'W' => [0x7f,0x20,0x18,0x20,0x7f], 'X' => [0x63,0x14,0x08,0x14,0x63],
        'Y' => [0x03,0x04,0x78,0x04,0x03], 'Z' => [0x61,0x51,0x49,0x45,0x43],
        '0' => [0x3e,0x51,0x49,0x45,0x3e], '1' => [0x00,0x42,0x7f,0x40,0x00],
        '2' => [0x42,0x61,0x51,0x49,0x46], '3' => [0x21,0x41,0x45,0x4b,0x31],
        '4' => [0x18,0x14,0x12,0x7f,0x10], '5' => [0x27,0x45,0x45,0x45,0x39],
        '6' => [0x3c,0x4a,0x49,0x49,0x30], '7' => [0x01,0x71,0x09,0x05,0x03],
        '8' => [0x36,0x49,0x49,0x49,0x36], '9' => [0x06,0x49,0x49,0x29,0x1e],
        ' ' => [0;5], '.' => [0,0x60,0x60,0,0], ':' => [0,0x36,0x36,0,0],
        '-' => [0x08,0x08,0x08,0x08,0x08], '=' => [0x14,0x14,0x14,0x14,0x14],
        '/' => [0x20,0x10,0x08,0x04,0x02], '_' => [0x40,0x40,0x40,0x40,0x40],
        '[' => [0,0x7f,0x41,0x41,0], ']' => [0,0x41,0x41,0x7f,0],
        '(' => [0,0x1c,0x22,0x41,0], ')' => [0,0x41,0x22,0x1c,0],
        '#' => [0x14,0x7f,0x14,0x7f,0x14], '!' => [0,0,0x5f,0,0],
        '?' => [0x02,0x01,0x51,0x09,0x06], ',' => [0,0x50,0x30,0,0],
        '+' => [0x08,0x08,0x3e,0x08,0x08], '*' => [0x14,0x08,0x3e,0x08,0x14],
        '\'' => [0,0x05,0x03,0,0], '"' => [0x03,0,0x03,0,0],
        _ => [0x02,0x01,0x51,0x09,0x06],
    }
}
