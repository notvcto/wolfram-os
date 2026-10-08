//! Freestanding shims for compiling with the installed Linux-host core.
//! The real x86_64-unknown-none target builds core without these libc imports.

use core::arch::asm;
use core::ffi::c_void;

#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut c_void, src: *const c_void, count: usize) -> *mut c_void {
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
pub unsafe extern "C" fn memset(dst: *mut c_void, value: i32, count: usize) -> *mut c_void {
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
pub unsafe extern "C" fn memmove(
    dst: *mut c_void,
    src: *const c_void,
    count: usize,
) -> *mut c_void {
    if (dst as usize) <= (src as usize) || (dst as usize) >= (src as usize).saturating_add(count) {
        return memcpy(dst, src, count);
    }
    let dst = dst as *mut u8;
    let src = src as *const u8;
    for i in (0..count).rev() {
        dst.add(i).write_volatile(src.add(i).read_volatile());
    }
    dst as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn bcmp(left: *const c_void, right: *const c_void, count: usize) -> i32 {
    let left = left as *const u8;
    let right = right as *const u8;
    for i in 0..count {
        if left.add(i).read_volatile() != right.add(i).read_volatile() {
            return 1;
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(left: *const c_void, right: *const c_void, count: usize) -> i32 {
    let left = left as *const u8;
    let right = right as *const u8;
    for i in 0..count {
        let a = left.add(i).read_volatile();
        let b = right.add(i).read_volatile();
        if a != b {
            return a as i32 - b as i32;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}
