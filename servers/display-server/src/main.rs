#![no_std]
#![no_main]

use log::info;

extern crate alloc;
extern crate libruntime;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    info!("Display server started");

    for (name, value) in libruntime::process::SelfProcess::get().args_all() {
        info!("Arg: {} = {}", name, value);
    }

    loop {
        libruntime::time::sleep(libruntime::time::Duration::from_seconds(1));
    }
}
