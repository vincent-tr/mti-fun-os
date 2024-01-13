#![no_std]

use core::hint::unreachable_unchecked;

use libsyscalls::process;

extern crate alloc;

mod allocator;
pub mod kobject;
mod logging;
mod panic;

pub fn init() {
    logging::init();
    kobject::init();
}

pub fn terminate() {
    kobject::terminate();
}

pub fn exit() {
    terminate();

    process::exit().expect("Could not exit process");
    unsafe { unreachable_unchecked() };
}
