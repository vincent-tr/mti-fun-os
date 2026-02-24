#![no_std]
#![no_main]

use libruntime::time::iface::build_ipc_server;

use crate::server::Server;

extern crate alloc;
extern crate libruntime;

mod rtc;
mod server;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let server = Server::new();
    let ipc_server = build_ipc_server(server).expect("failed to build time-server IPC server");

    ipc_server.run()
}
