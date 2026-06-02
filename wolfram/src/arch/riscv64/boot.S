    .section .text.entry
    .globl _start

_start:
    # set up stack pointer
    # each hart gets 64KB of stack
    la sp, boot_stack_top
    call kernel_main

    # should never reach here
spin:
    wfi
    j spin

    .section .bss.stack
    .globl boot_stack_bottom
boot_stack_bottom:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:
