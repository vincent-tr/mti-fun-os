#![no_std]
#![no_main]

use libruntime::{service, time::iface::setup_ipc_server};

use crate::server::Server;

extern crate alloc;
extern crate libruntime;

mod rtc;
mod server;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let server = Server::new();
    let runner = service::Runner::new();
    setup_ipc_server(server, &runner).expect("failed to build time-server IPC server");

    runner.run()
}
