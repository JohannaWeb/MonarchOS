/* Multiboot2 and PVH headers for x86_64 */

.set MULTIBOOT2_MAGIC, 0xe85250d6
.set MULTIBOOT_ARCH_I386, 0
.set MULTIBOOT_TAG_ALIGN, 8
.set MULTIBOOT_TAG_END, 0
.set NT_PVH, 0x100

/* PVH note - format: ELF NOTE with namesz, descsz, type, name, desc */
.section .note.pvh, "a"
.align 4
    .long 0x4              /* namesz - "PVH\0" is 4 bytes */
    .long 0x4              /* descsz - version field is 4 bytes */
    .long NT_PVH           /* type = NT_PVH (0x100) */
    .ascii "PVH\0"         /* name */
.align 4
    .long 0                /* PVH version 0 */
.align 4

/* Multiboot2 header */
.section .multiboot_header
.align MULTIBOOT_TAG_ALIGN
multiboot_header_start:
    .long MULTIBOOT2_MAGIC
    .long MULTIBOOT_ARCH_I386
    .long multiboot_header_end - multiboot_header_start
    .long -(MULTIBOOT2_MAGIC + MULTIBOOT_ARCH_I386 + (multiboot_header_end - multiboot_header_start))

    /* End tag */
    .align MULTIBOOT_TAG_ALIGN
    .short MULTIBOOT_TAG_END
    .short 0
    .long 8
multiboot_header_end:
