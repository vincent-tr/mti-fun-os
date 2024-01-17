use core::ops::Range;

// Defined by linker script
extern "C" {
    // overall
    static __start: u8;
    static __end: u8;

    // text (R-X)
    static __text_start: u8;
    static __text_end: u8;
    // rodata (R--)
    static __rodata_start: u8;
    static __rodata_end: u8;
    // data (RW-)
    static __data_start: u8;
    static __data_end: u8;
    static __bss_start: u8;
    static __bss_end: u8;

    // stack in RW data
    static __init_stack_start: u8;
    pub static __init_stack_end: u8;

    // idle in text
    static __idle_start: u8;
    static __idle_end: u8;
}

pub fn global() -> Range<usize> {
    unsafe {
        let start = &__start as *const u8 as usize;
        let end = &__end as *const u8 as usize;
        start..end
    }
}

pub fn text() -> Range<usize> {
    unsafe {
        let start = &__text_start as *const u8 as usize;
        let end = &__text_end as *const u8 as usize;
        start..end
    }
}

pub fn rodata() -> Range<usize> {
    unsafe {
        let start = &__rodata_start as *const u8 as usize;
        let end = &__rodata_end as *const u8 as usize;
        start..end
    }
}

pub fn data() -> Range<usize> {
    unsafe {
        let start = &__data_start as *const u8 as usize;
        let end = &__data_end as *const u8 as usize;
        start..end
    }
}

// __init_stack_end is directly used in bootstrap asm
#[allow(dead_code)]
pub fn stack_top() -> usize {
    unsafe { &__init_stack_end as *const u8 as usize }
}

pub fn idle() -> Range<usize> {
    unsafe {
        let start = &__idle_start as *const u8 as usize;
        let end = &__idle_end as *const u8 as usize;
        start..end
    }
}
