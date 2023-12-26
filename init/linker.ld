ENTRY(main)
OUTPUT_FORMAT(elf64-x86-64)

INIT_BASE_ADDRESS = 0x200000;
INIT_STACK_SIZE = 0x5000;

SECTIONS {
    . = INIT_BASE_ADDRESS;
    . += SIZEOF_HEADERS;

    .text ALIGN(4K) : {
        __text_start = .;
        *(.text*)
        . = ALIGN(4K);
        __text_end = .;
    }

    .rodata ALIGN(4K) : {
        __rodata_start = .;
        *(.rodata*)
        . = ALIGN(4K);
        __rodata_end = .;
    }

    .data ALIGN(4K) : {
        __bss_start = .;
        *(.bss*)
        . = ALIGN(4K);
        __bss_end = .;
        __init_stack_start = .;
        . += INIT_STACK_SIZE;
        . = ALIGN(4096);
        __init_stack_end = .;
        __data_start = .;
        *(.data*)
        . = ALIGN(4K);
        __data_end = .;
        
    }

    __end = .;

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
        *(.rel.eh_frame*)
    }
}
