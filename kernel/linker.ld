/*
 * Project Name:  BismuthOS
 * File Name:     linker.ld
 * File Function: Linker script of kernel
 * Author:        
 * License:       MIT License
 */

ENTRY(_start)

SECTIONS {
    . = 0x00100000;

    _kernel_start = .;

    .start : {
        *(.start)
    }

    .text : {
        *(.text .text.*)
    }

    .bss : {
        *(.bss .bss.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .eh_frame : {
        *(.eh_frame .eh_frame.*)
    }

    .eh_frame_hdr : {
        *(.eh_frame_hdr .eh_frame_hdr.*)
    }

    . = _kernel_start + 0x00100000 - 2;

    .end_marker :
    {
        SHORT(0xDEAD)
    }
}