#![no_std]
#![no_main]

extern crate libruntime;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    log::info!("e1000e-server starting...");

    // TODO: Implement E1000e network driver

    loop {
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}
