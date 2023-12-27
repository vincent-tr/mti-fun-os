#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

mod logging;
mod offsets;

use core::{arch::asm, panic::PanicInfo};

use log::{error, info};

// Special init start: need to setup its own stack
#[naked]
#[no_mangle]
#[link_section = ".text_entry"]
pub unsafe extern "C" fn user_start() {
    asm!(
        "
        lea rsp, {stack}
        mov rbp, rsp

        call {main}
        # `main` must never return.
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
        "text: 0x{:016X} -> 0x{:016X} (size=0x{:X})",
        offsets::text().start,
        offsets::text().end,
        offsets::text().end - offsets::text().start
    );
    info!(
        "rodata: 0x{:016X} -> 0x{:016X} (size=0x{:X})",
        offsets::rodata().start,
        offsets::rodata().end,
        offsets::rodata().end - offsets::rodata().start
    );
    info!(
        "data: 0x{:016X} -> 0x{:016X} (size=0x{:X})",
        offsets::data().start,
        offsets::data().end,
        offsets::data().end - offsets::data().start
    );
    info!("stack_top: 0x{:016X}", offsets::stack_top());

    info!("test");

    {
        let handle = libsyscalls::process::open_self().expect("Could not open handle");

        info!("handle value={handle:?}");
    }

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info}");

    loop {}
}

/*
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
*/
