//! Wolfram Kernel Panic Handler
//!
//! Informative. Honest. Occasionally funny.
//! Always ends with W —
//!
//! The panic count exists so you know it's not just you.

use crate::kprintln;
use core::sync::atomic::{AtomicU32, Ordering};

static PANIC_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn kernel_panic(info: &core::panic::PanicInfo) -> ! {
    let count = PANIC_COUNT.fetch_add(1, Ordering::Relaxed) + 1;

    kprintln!();
    kprintln!("════════════════════════════════════════════");
    kprintln!("  W O L F R A M   K E R N E L   P A N I C  ");
    kprintln!("════════════════════════════════════════════");
    kprintln!();

    let s = fmt_message(info);
    let msg = classify(&s);
    kprintln!("  {}", msg);
    kprintln!();
    kprintln!("────────────────────────────────────────────");

    if let Some(loc) = info.location() {
        kprintln!("  where:       {}:{}", loc.file(), loc.line());
    }

    kprintln!("  panic count: {} (including this one)", count);
    kprintln!();
    kprintln!("  github.com/notvcto/wolfram-os/issues");
    kprintln!();
    kprintln!("W — {}", sign_off(&s));
    kprintln!("════════════════════════════════════════════");

    loop {
        core::hint::spin_loop();
    }
}

fn classify(s: &StackStr) -> &'static str {
    if s.contains("spawn init") {
        return "init does not exist yet.\n  \
                that's expected. that's Phase 1.\n  \
                the kernel booted. serial works. panic screen works.\n  \
                day one.";
    }
    if s.contains("oom") || s.contains("allocate") {
        return "we ran out of memory.\n  \
                where the hell is your swap?";
    }
    if s.contains("capability") || s.contains("handle") {
        return "the capability validator lost the plot.\n  \
                this is the worst possible fault.\n  \
                please report immediately.";
    }
    if s.contains("stack") {
        return "the stack overflowed.\n  \
                it kept going. and going.\n  \
                check for infinite recursion.";
    }
    "something went wrong.\n  \
     the information above is everything we have."
}

fn sign_off(s: &StackStr) -> &'static str {
    if s.contains("spawn init") { return "it begins."; }
    if s.contains("capability") { return "we owe you one."; }
    if s.contains("stack")      { return "turtles all the way down."; }
    if s.contains("oom")        { return "memory is finite. who knew."; }
    "we'll figure out what we did."
}

// format the panic message into a fixed stack buffer — no allocator needed
fn fmt_message(info: &core::panic::PanicInfo) -> StackStr {
    let mut s = StackStr::new();
    use core::fmt::Write;
    let _ = write!(s, "{}", info.message());
    s
}

struct StackStr {
    buf: [u8; 256],
    len: usize,
}

impl StackStr {
    fn new() -> Self { Self { buf: [0u8; 256], len: 0 } }

    fn contains(&self, needle: &str) -> bool {
        let s = core::str::from_utf8(&self.buf[..self.len]).unwrap_or("");
        s.contains(needle)
    }
}

impl core::fmt::Write for StackStr {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            if self.len < self.buf.len() {
                self.buf[self.len] = b;
                self.len += 1;
            }
        }
        Ok(())
    }
}
