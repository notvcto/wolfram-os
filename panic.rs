//! Wolfram Kernel Panic Handler
//!
//! If you're here because you hit one of these screens:
//! welcome. you're in good company. this happens a lot during development.
//! the panic count field exists specifically so you know it's not just you.
//!
//! ~liedtke would say the panic handler itself should never panic.
//! liedtke would be right. we're working on it.

use crate::arch::serial;

/// The panic screen format.
/// Informative. Honest. Occasionally funny.
/// Always ends with W —
pub fn kernel_panic(info: &PanicInfo) -> ! {
    serial::println!("════════════════════════════════════════════");
    serial::println!("  W O L F R A M   K E R N E L   P A N I C  ");
    serial::println!("════════════════════════════════════════════");
    serial::println!();

    // pick the message based on what went wrong
    let message = classify_panic(info);
    serial::println!("  {}", message);
    serial::println!();
    serial::println!("────────────────────────────────────────────");

    if let Some(location) = info.location() {
        serial::println!("  where:       {}:{}", location.file(), location.line());
    }

    serial::println!("  panic count: {} (including this one)", PANIC_COUNT.fetch_add(1, Ordering::Relaxed) + 1);
    serial::println!();
    serial::println!("  please report this at:");
    serial::println!("  github.com/notvcto/wolfram-os/issues");
    serial::println!();
    serial::println!("W — {}", sign_off(info));
    serial::println!("════════════════════════════════════════════");

    loop {
        core::hint::spin_loop();
    }
}

fn classify_panic(info: &PanicInfo) -> &'static str {
    let msg = info.message().map(|m| format!("{}", m)).unwrap_or_default();

    if msg.contains("capability") || msg.contains("handle") {
        "the capability validator lost the plot.\n  \
         this is the worst possible fault.\n  \
         this is our fault, not yours.\n  \
         please report immediately."
    } else if msg.contains("stack overflow") || msg.contains("stack") {
        "the stack overflowed.\n  \
         it kept going. and going.\n  \
         then it ran into something it wasn't supposed to.\n  \
         check for infinite recursion. or just very deep recursion.\n  \
         probably one of those."
    } else if msg.contains("oom") || msg.contains("out of memory") || msg.contains("alloc") {
        "we ran out of memory.\n  \
         all of it. every last byte.\n  \
         where the hell is your swap?\n  \
         (not that swap fixes bad code. but it buys time.)"
    } else if msg.contains("divide") || msg.contains("division") {
        "something divided by zero.\n  \
         mathematically undefined.\n  \
         philosophically troubling.\n  \
         computationally fatal."
    } else if msg.contains("panic handler") {
        "the panic handler panicked.\n  \
         .\n  \
         ..\n  \
         ...\n  \
         we don't have words for this."
    } else {
        "something went wrong.\n  \
         we're not sure what.\n  \
         the information above is everything we have.\n  \
         it may not be enough. sorry about that."
    }
}

fn sign_off(info: &PanicInfo) -> &'static str {
    let msg = info.message().map(|m| format!("{}", m)).unwrap_or_default();

    if msg.contains("capability") {
        "we owe you one."
    } else if msg.contains("stack") {
        "it's turtles all the way down, apparently."
    } else if msg.contains("oom") || msg.contains("alloc") {
        "memory is finite. who knew."
    } else if msg.contains("divide") {
        "some lessons are learned the hard way."
    } else if msg.contains("panic handler") {
        "we'll have a long think about this one."
    } else {
        "we'll figure out what we did."
    }
}

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU32, Ordering};

static PANIC_COUNT: AtomicU32 = AtomicU32::new(0);
