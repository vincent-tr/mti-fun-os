#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod buffer_pool;
mod iface;
mod server;

use libruntime::{r#async, net::iface::build_ipc_server};

use crate::server::Server;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    log::info!("net-server starting...");

    buffer_pool::init();

    let server = Server::new();
    let (ipc_server, ipc_process_termination_listener) =
        build_ipc_server(server).expect("failed to build net-server IPC server");

    ipc_server.start();
    ipc_process_termination_listener.start();

    r#async::block_on();

    // Server should never complete
    unreachable!();
}
