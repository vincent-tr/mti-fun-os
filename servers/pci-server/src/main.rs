#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod access;
mod device;
mod server;
mod state;

use libruntime::drivers::pci::iface::build_ipc_server;

use crate::server::Server;

// https://wiki.osdev.org/PCI

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let server = Server::new();
    let ipc_server = build_ipc_server(server).expect("failed to build time-server IPC server");

    ipc_server.run()
}
