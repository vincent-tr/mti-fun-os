#![no_std]
#![no_main]

use log::info;

extern crate alloc;
extern crate libruntime;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    info!("Display server started");

    loop {
        libruntime::timer::sleep(libruntime::timer::Duration::from_seconds(1));
    }
}
