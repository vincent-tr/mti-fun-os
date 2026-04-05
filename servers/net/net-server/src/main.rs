#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod buffer_pool;
mod iface;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    log::info!("net-server starting...");

    buffer_pool::init();

    // TODO: Implement network stack

    loop {
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}
