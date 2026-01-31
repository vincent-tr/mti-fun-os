#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;
extern crate libruntime;

mod manager;

use manager::Manager;

#[no_mangle]
pub fn main() {
    let manager = Manager::new().expect("failed to create process-server");

    let server = manager
        .build_ipc_server()
        .expect("failed to build process-server IPC server");

    server.run();
}
