/* Kernel linker script for x86_64 */

ENTRY(_start)

SECTIONS {
    . = 0xFFFFFFFF80000000;

    .text : AT(ADDR(.text) - 0xFFFFFFFF80000000) {
        *(.text .text.*)
        . = ALIGN(4K);
    }

    .rodata : AT(ADDR(.rodata) - 0xFFFFFFFF80000000) {
        *(.rodata .rodata.*)
        . = ALIGN(4K);
    }

    .data : AT(ADDR(.data) - 0xFFFFFFFF80000000) {
        *(.data .data.*)
        . = ALIGN(4K);
    }

    .bss : AT(ADDR(.bss) - 0xFFFFFFFF80000000) {
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4K);
    }
}
