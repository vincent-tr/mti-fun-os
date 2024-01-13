#![no_std]

use core::hint::unreachable_unchecked;

use libsyscalls::process;
use log::debug;

extern crate alloc;

mod allocator;
pub mod kobject;
mod logging;
mod panic;

pub fn init() {
    logging::init();
    debug!("init");

    kobject::init();
}

pub fn terminate() {
    kobject::terminate();
}

pub fn exit() -> ! {
    debug!("exit");
    terminate();

    process::exit().expect("Could not exit process");
    unsafe { unreachable_unchecked() };
}
