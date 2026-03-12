#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod device;
mod pci;
mod server;
mod state;

use libruntime::{drivers::pci::iface::setup_ipc_server, service};

use crate::{pci::ConfigurationSpace, server::Server};

// https://wiki.osdev.org/PCI

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    unsafe { ConfigurationSpace::init() };

    let server = Server::new();
    let runner = service::Runner::new();
    setup_ipc_server(server, &runner).expect("failed to build PCI IPC server");

    runner.run()
}
