#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod descriptors;
mod device;
mod eeprom;
mod link_status;
mod registers;
mod rx_ring;
mod tx_ring;

use lazy_static::lazy_static;
use libruntime::{net::dev::setup_net_device_server, service};

use device::E1000eDevice;

lazy_static! {
    pub static ref RUNNER: service::Runner = service::Runner::new();
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    setup_net_device_server::<E1000eDevice>("net.dev.e1000e", &RUNNER)
        .expect("failed to build net.dev.e1000e IPC server");

    RUNNER.run()
}
