#[repr(C)]
pub struct SyscallFrame {
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub r10: u64, // r10 replaces rcx in the syscall ABI
    pub r8: u64,
    pub r9: u64,
    pub rax: u64, // syscall number
    pub rcx: u64, // saved rip (by SYSCALL instruction)
    pub r11: u64, // saved rflags (by SYSCALL instruction)
}
