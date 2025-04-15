#![no_std]
#![allow(internal_features)]
#![feature(panic_internals)]
#![feature(never_type)]
#![feature(let_chains)]

use core::hint::unreachable_unchecked;

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
