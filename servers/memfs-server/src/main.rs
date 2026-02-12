#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

#[no_mangle]
pub fn main() -> i32 {
    log::info!("Hello world!");

    0
}
