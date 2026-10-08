    .section .text, "ax"

    .macro VECTOR number has_error
    .global x86_vector_\number
x86_vector_\number:
    .if !\has_error
        pushq $0
    .endif
    pushq $\number
    jmp x86_trap_common
    .endm

    VECTOR 0,0
    VECTOR 1,0
    VECTOR 2,0
    VECTOR 3,0
    VECTOR 4,0
    VECTOR 5,0
    VECTOR 6,0
    VECTOR 7,0
    VECTOR 8,1
    VECTOR 9,0
    VECTOR 10,1
    VECTOR 11,1
    VECTOR 12,1
    VECTOR 13,1
    VECTOR 14,1
    VECTOR 15,0
    VECTOR 16,0
    VECTOR 17,1
    VECTOR 18,0
    VECTOR 19,0
    VECTOR 20,0
    VECTOR 21,1
    VECTOR 22,0
    VECTOR 23,0
    VECTOR 24,0
    VECTOR 25,0
    VECTOR 26,0
    VECTOR 27,0
    VECTOR 28,0
    VECTOR 29,1
    VECTOR 30,1
    VECTOR 31,0

    .global x86_unexpected_interrupt
x86_unexpected_interrupt:
    pushq $0
    pushq $255

x86_trap_common:
    cld
    movq %rsp, %rdi
    andq $-16, %rsp
    call x86_trap_handler
1:
    hlt
    jmp 1b

    .section .rodata, "a"
    .global x86_trap_vectors
x86_trap_vectors:
    .quad x86_vector_0, x86_vector_1, x86_vector_2, x86_vector_3
    .quad x86_vector_4, x86_vector_5, x86_vector_6, x86_vector_7
    .quad x86_vector_8, x86_vector_9, x86_vector_10, x86_vector_11
    .quad x86_vector_12, x86_vector_13, x86_vector_14, x86_vector_15
    .quad x86_vector_16, x86_vector_17, x86_vector_18, x86_vector_19
    .quad x86_vector_20, x86_vector_21, x86_vector_22, x86_vector_23
    .quad x86_vector_24, x86_vector_25, x86_vector_26, x86_vector_27
    .quad x86_vector_28, x86_vector_29, x86_vector_30, x86_vector_31
