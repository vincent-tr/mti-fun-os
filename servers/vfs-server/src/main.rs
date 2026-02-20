#![no_std]
#![no_main]
#![feature(let_chains)]

extern crate alloc;
extern crate libruntime;

mod cache;
mod lookup;
mod mounts;
mod opened_node;
mod server;
mod state;
mod vnode;

use libruntime::vfs::iface::build_ipc_server;

use crate::server::Server;

#[no_mangle]
pub fn main() -> i32 {
    let server = Server::new();
    let ipc_server = build_ipc_server(server).expect("failed to build vfs-server IPC server");

    ipc_server.run()
}
