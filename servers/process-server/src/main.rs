#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;
extern crate libruntime;

mod error;
mod loader;
mod manager;
mod process;

use manager::Manager;

// Note: process server should not use libruntime Process API:
// - During initialization, the process-server (self) will not be up.
// - After initialization, calling itself in a sync way will result in a deadlock.

#[no_mangle]
pub fn main() -> i32 {
    let manager = Manager::new().expect("failed to create process-server");

    let server = manager
        .build_ipc_server()
        .expect("failed to build process-server IPC server");

    server.run()
}
