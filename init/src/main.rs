#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

mod logging;
mod syscalls;

use core::{arch::asm, fmt, mem, panic::PanicInfo};

use log::{error, info};
use syscalls::{syscall1, syscall3, SyscallNumber};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Handle(u64);

impl Handle {
    pub const fn invalid() -> Self {
        Handle(0)
    }
}

/// # Safety
///
/// Borrowing rules unchecked. Do right before syscalls only.
unsafe fn out_ptr<T>(value: &mut T) -> usize {
    let ptr: *mut T = value;
    mem::transmute(ptr)
}

mod offsets {
    use core::ops::Range;

    extern "C" {
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

        static __end: u8;

        // stack in RW data
        static __init_stack_start: u8;
        pub static __init_stack_end: u8;
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

    pub fn stack_top() -> usize {
        unsafe { &__init_stack_end as *const u8 as usize }
    }
}

// Special init start: need to setup its own stack
#[naked]
#[no_mangle]
#[link_section = ".text_entry"]
pub unsafe extern "C" fn user_start() {
    core::arch::asm!(
        "
        lea rsp, {stack}
        mov rbp, rsp

        call {main}
        # `start` must never return.
        ud2
        ",
        stack = sym offsets::__init_stack_end,
        main = sym main,
        options(noreturn),
    );
}

// Force at least one data, so that it is laid out after bss in linker script
// This force bss allocation in binary file
#[used(linker)]
static mut FORCE_DATA_SECTION: u8 = 0x42;

extern "C" fn main() -> ! {
    logging::init();

    // TODO: protection
    info!(
        "text: {:016X} -> {:016X} (size={})",
        offsets::text().start,
        offsets::text().end,
        offsets::text().end - offsets::text().start
    );
    info!(
        "rodata: {:016X} -> {:016X} (size={})",
        offsets::rodata().start,
        offsets::rodata().end,
        offsets::rodata().end - offsets::rodata().start
    );
    info!(
        "data: {:016X} -> {:016X} (size={})",
        offsets::data().start,
        offsets::data().end,
        offsets::data().end - offsets::data().start
    );
    info!("stack_top: {:016X}", offsets::stack_top());

    info!("test");

    unsafe {
        let mut handle = Handle::invalid();
        syscall1(SyscallNumber::ProcessOpenSelf, out_ptr(&mut handle));

        info!("handle value={handle:?}");

        syscall1(SyscallNumber::Close, handle.0 as usize);
    }

    loop {}
}

#[inline]
fn debugbreak() {
    unsafe {
        asm!("int3", options(nomem, nostack));
    }
}

#[inline]
fn page_fault() {
    let ptr = 0x42 as *mut u8;
    unsafe { *ptr = 42 };
}

#[allow(unconditional_panic)]
#[inline]
fn div0() {
    // div / 0
    let _ = 42 / 0;
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info}");

    loop {}
}
