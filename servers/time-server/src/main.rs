#![no_std]
#![no_main]

use log::info;

extern crate alloc;
extern crate libruntime;

#[no_mangle]
pub fn main() -> i32 {
    info!("Time server started");
    0
}
