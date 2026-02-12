#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

use crate::manager::Manager;

extern crate alloc;
extern crate libruntime;

mod error;
mod manager;

#[no_mangle]
pub fn main() -> i32 {
    let manager = Manager::new().expect("failed to create vfs-server");

    let server = manager
        .build_ipc_server()
        .expect("failed to build vfs-server IPC server");

    server.run()
}
