//! Freestanding shims for compiling with the installed Linux-host core.
//! The real x86_64-unknown-none target builds core without these libc imports.

use core::arch::asm;

#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, count: usize) -> *mut u8 {
    asm!(
        "cld",
        "rep movsb",
        inout("rdi") dst => _,
        inout("rsi") src => _,
        inout("rcx") count => _,
        options(nostack),
    );
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memset(dst: *mut u8, value: i32, count: usize) -> *mut u8 {
    asm!(
        "cld",
        "rep stosb",
        inout("rdi") dst => _,
        in("al") value as u8,
        inout("rcx") count => _,
        options(nostack),
    );
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, count: usize) -> *mut u8 {
    if (dst as usize) <= (src as usize) || (dst as usize) >= (src as usize).saturating_add(count) {
        return memcpy(dst, src, count);
    }
    for i in (0..count).rev() {
        dst.add(i).write_volatile(src.add(i).read_volatile());
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn bcmp(left: *const u8, right: *const u8, count: usize) -> i32 {
    for i in 0..count {
        if left.add(i).read_volatile() != right.add(i).read_volatile() { return 1; }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(left: *const u8, right: *const u8, count: usize) -> i32 {
    for i in 0..count {
        let a = left.add(i).read_volatile();
        let b = right.add(i).read_volatile();
        if a != b { return a as i32 - b as i32; }
    }
    0
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}
