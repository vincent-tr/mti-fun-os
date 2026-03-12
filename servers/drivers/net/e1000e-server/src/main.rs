#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod device;
mod registers;

use libruntime::{net::dev::setup_net_device_server, service};

use device::E1000eDevice;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let runner = service::Runner::new();
    setup_net_device_server::<E1000eDevice>("net.dev.e1000e", &runner)
        .expect("failed to build net.dev.e1000e IPC server");

    runner.run()
}
