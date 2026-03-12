#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod device;
mod pci;
mod server;
mod state;

use libruntime::drivers::pci::iface::build_ipc_runner;

use crate::{pci::ConfigurationSpace, server::Server};

// https://wiki.osdev.org/PCI

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    unsafe { ConfigurationSpace::init() };

    let server = Server::new();
    let ipc_runner = build_ipc_runner(server).expect("failed to build PCI server IPC runner");

    ipc_runner.run()
}
