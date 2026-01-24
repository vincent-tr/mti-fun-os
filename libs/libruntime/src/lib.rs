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
}

pub fn exit() -> ! {
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
pub fn main() -> ! {
    // TODO: args:
    // - init function array
    // - fini function array
    // - program entry point
    // will be passed by the linker into a mapped memory page.
    // arg will be the address of the mapped memory page.
    // This api has to free the mapping after use.

    panic("TODO: libruntime::main()");
}
