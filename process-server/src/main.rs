#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;
extern crate libruntime;

use log::info;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    libruntime::init();

    info!("Hello, world!");

    libruntime::exit()
}
