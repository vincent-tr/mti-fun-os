#![no_std]
#![no_main]

use libruntime::time::iface::build_ipc_runner;

use crate::server::Server;

extern crate alloc;
extern crate libruntime;

mod rtc;
mod server;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let server = Server::new();
    let ipc_runner = build_ipc_runner(server).expect("failed to build time-server IPC server");

    ipc_runner.run()
}
