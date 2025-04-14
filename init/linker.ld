ENTRY(user_start)
OUTPUT_FORMAT(elf64-x86-64)

INIT_BASE_ADDRESS = 0x200000;
INIT_STACK_SIZE = 0x2000;

SECTIONS {
    . = INIT_BASE_ADDRESS;
    PROVIDE(__start = .);
    . += SIZEOF_HEADERS;

    .text ALIGN(4K) : {
        PROVIDE(__text_start = .);

        // Force entry point at the begining of text section
        *(.text_entry)
        PROVIDE(__idle_start = .);
        *(.text_idle)
        PROVIDE(__idle_end = .);
        KEEP(*(.text*))
        
        . = ALIGN(4K);
        PROVIDE(__text_end = .);
    }

    .rodata ALIGN(4K) : {
        PROVIDE(__rodata_start = .);

        KEEP(*(.rodata*))

        . = ALIGN(4K);
        PROVIDE(__rodata_end = .);
    }

    .data ALIGN(4K) : {
        PROVIDE(__data_start = .);

        *(.bss*)

        PROVIDE(__init_stack_start = .);
        . = ALIGN(4K);
        . += INIT_STACK_SIZE;
        . = ALIGN(4K);
        PROVIDE(__init_stack_end = .);

        KEEP(*(.data*))

        . = ALIGN(4K);
        PROVIDE(__data_end = .);
    }

     PROVIDE(__end = .);

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
        *(.rel.eh_frame*)
    }
}
