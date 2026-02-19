#![no_std]
#![no_main]
#![feature(let_chains)]

extern crate alloc;
extern crate libruntime;

mod instance;
mod server;
mod state;

use libruntime::vfs::fs::iface::build_ipc_server;

use crate::server::Server;

#[no_mangle]
pub fn main() -> i32 {
    let server = Server::new();
    let ipc_server =
        build_ipc_server(server, "memfs-server").expect("failed to build memfs-server IPC server");

    ipc_server.run()
}
