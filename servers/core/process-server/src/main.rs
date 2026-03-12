#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod error;
mod loader;
mod process;
mod server;
mod state;

use libruntime::{process::iface::setup_ipc_server, service};
use server::Server;

// Note: process server should not use libruntime Process API:
// - During initialization, the process-server (self) will not be up.
// - After initialization, calling itself in a sync way will result in a deadlock.

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let server = Server::new().expect("failed to create process-server");
    let runner = service::Runner::new();
    setup_ipc_server(server, &runner).expect("failed to build process-server IPC server");

    runner.run()
}
