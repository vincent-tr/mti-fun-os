#![no_std]
#![allow(internal_features)]
#![feature(panic_internals)]
#![feature(never_type)]
#![feature(let_chains)]

use core::{hint::unreachable_unchecked, panicking::panic};

use libsyscalls::process;
use log::debug;

extern crate alloc;

mod allocator;
pub mod debug;
pub mod kobject;
mod logging;
pub mod sync;

pub fn init() {
    logging::init();
    debug!("init");

    kobject::init();

    // run global constructors
    unsafe {
        let init_array = make_array(&__init_array_start, &__init_array_end);
        for constructor in init_array {
            constructor();
        }
    }
}

pub fn exit() -> ! {
    // run global destructors
    unsafe {
        let fini_array = make_array(&__fini_array_start, &__fini_array_end);
        for destructor in fini_array.iter().rev() {
            destructor();
        }
    }

    debug!("exit");
    kobject::terminate();

    process::exit().expect("Could not exit process");
    unsafe { unreachable_unchecked() };
}

/// Program entry point
/// This entry is expected to run
/// - init()
/// - library constructors
/// - actual program entry point `main()`
/// - library destructors
/// - exit()
extern "C" fn runtime_entry() -> ! {
    // TODO: args:
    // - init function array
    // - fini function array
    // - program entry point
    // will be passed by the linker into a mapped memory page.
    // arg will be the address of the mapped memory page.
    // This api has to free the mapping after use.

    panic("TODO: libruntime::main()");
}

// Defined by linker script
extern "C" {
    // init/fini array in text
    static __init_array_start: u8;
    static __init_array_end: u8;
    static __fini_array_start: u8;
    static __fini_array_end: u8;
}

unsafe fn make_array(start: &u8, end: &u8) -> &'static [fn()] {
    let start = start as *const u8 as *const fn();
    let end = end as *const u8 as *const fn();

    let count = end.offset_from(start) as usize;
    core::slice::from_raw_parts(start, count)
}
