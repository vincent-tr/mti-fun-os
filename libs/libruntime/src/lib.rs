#![no_std]
#![allow(internal_features)]
#![feature(panic_internals)]
#![feature(never_type)]
#![feature(str_as_str)]

use core::hint::unreachable_unchecked;

use log::debug;

extern crate alloc;

mod allocator;
pub mod r#async;
pub mod collections;
pub mod debug;
#[cfg(feature = "entry")]
mod entry;
pub mod file;
pub mod ipc;
pub mod kobject;
mod logging;
pub mod memory;
pub mod process;
pub mod state;
pub mod sync;
pub mod time;

pub unsafe fn init() {
    logging::init();
    debug!("init");

    unsafe {
        // run global constructors
        let init_array = make_array(&__init_array_start, &__init_array_end);
        for constructor in init_array {
            constructor();
        }

        kobject::init();

        #[cfg(feature = "init-process")]
        process::init();
    }
}

pub unsafe fn exit() -> ! {
    // run global destructors
    unsafe {
        kobject::exit();

        let fini_array = make_array(&__fini_array_start, &__fini_array_end);
        for destructor in fini_array.iter().rev() {
            destructor();
        }
    }

    debug!("exit");
    libsyscalls::process::exit().expect("Could not exit process");
    unsafe { unreachable_unchecked() };
}

// Defined by linker script
unsafe extern "C" {
    // init/fini array in text
    static __init_array_start: u8;
    static __init_array_end: u8;
    static __fini_array_start: u8;
    static __fini_array_end: u8;
}

unsafe fn make_array(start: &u8, end: &u8) -> &'static [fn()] {
    let start = start as *const u8 as *const fn();
    let end = end as *const u8 as *const fn();

    unsafe {
        let count = end.offset_from(start) as usize;
        core::slice::from_raw_parts(start, count)
    }
}
