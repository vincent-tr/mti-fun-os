#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod device;
mod registers;

use libruntime::net::dev::build_net_device_runner;

use device::E1000eDevice;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let ipc_runner = build_net_device_runner::<E1000eDevice>("net.dev.e1000e")
        .expect("failed to build net.dev.e1000e IPC server");

    ipc_runner.run()
}
