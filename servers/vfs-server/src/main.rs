#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;
extern crate libruntime;

use log::info;

#[no_mangle]
pub fn main() {
    info!("Hello, world!");
}
