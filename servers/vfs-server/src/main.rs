#![no_std]
#![no_main]

use libruntime::vfs::iface::build_ipc_server;

use crate::server::Server;

extern crate alloc;
extern crate libruntime;

mod server;

#[no_mangle]
pub fn main() -> i32 {
    let server = Server::new().expect("failed to create vfs-server");
    let ipc_server = build_ipc_server(server).expect("failed to build vfs-server IPC server");

    ipc_server.run()
}
