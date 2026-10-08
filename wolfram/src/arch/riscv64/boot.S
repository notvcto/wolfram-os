    .section .text.entry
    .globl _start

_start:
    # Only the first hart may enter the single 64 KiB boot stack. The atomic
    # claim works even when the firmware's boot hart ID is not zero.
    la t0, boot_hart_claimed
    li t1, 1
    amoswap.w.aq t2, t1, (t0)
    bnez t2, secondary_hart_park

    la sp, boot_stack_top
    call kernel_main

    # should never reach here
spin:
    wfi
    j spin

secondary_hart_park:
    wfi
    j secondary_hart_park

    .section .bss
    .align 2
boot_hart_claimed:
    .word 0

    .section .bss.stack
    .align 4
    .globl boot_stack_bottom
boot_stack_bottom:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:
