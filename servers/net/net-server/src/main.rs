#![no_std]
#![no_main]

extern crate libruntime;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    log::info!("net-server starting...");

    // TODO: Implement network stack

    loop {
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}
