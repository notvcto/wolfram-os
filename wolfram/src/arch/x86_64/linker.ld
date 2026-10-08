ENTRY(_start)

SECTIONS {
    . = 0;
    __kernel_start = .;

    .text : ALIGN(4096) {
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    }
    .data : ALIGN(4096) {
        *(.data .data.*)
    }
    .bss : ALIGN(4096) {
        *(.bss .bss.*)
        *(COMMON)
    }

    . = ALIGN(4096);
    __kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}
