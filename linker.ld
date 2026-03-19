/* Kernel linker script for x86_64 with Multiboot2 support */

ENTRY(_start)

SECTIONS {
    . = 0x100000;
    __kernel_start = .;

    /* Multiboot2 header must be in first 32KB */
    .multiboot_header : {
        KEEP(*(.multiboot_header))
    }

    /* PVH note for QEMU */
    .note.pvh : {
        KEEP(*(.note.pvh))
    }

    .text : {
        *(.text .text.*)
        . = ALIGN(4K);
    }

    .rodata : {
        *(.rodata .rodata.*)
        . = ALIGN(4K);
    }

    .data : {
        *(.data .data.*)
        . = ALIGN(4K);
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4K);
    }

    /* Limine sections (if using limine bootloader) */
    .limine_requests_start ALIGN(8) : {
        KEEP(*(.limine_requests_start))
    }

    .limine_requests ALIGN(8) : {
        KEEP(*(.limine_requests))
    }

    .limine_requests_end ALIGN(8) : {
        KEEP(*(.limine_requests_end))
    }

    __kernel_end = ALIGN(4K);
}
