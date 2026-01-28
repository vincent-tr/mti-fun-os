#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;
extern crate libruntime;

use log::{error, info};

use libruntime::{ipc, process::messages};

#[no_mangle]
pub fn main() {
    let server = ipc::ServerBuilder::new(messages::PORT_NAME, messages::VERSION)
        .with_handler(messages::Type::CreateProcess, create_process)
        .build()
        .expect("failed to build process-server IPC server");

    info!("process-server started");

    server.run();
}

fn create_process(
    query: messages::CreateProcessQueryParameters,
    handles: ipc::Handles,
) -> Result<(messages::CreateProcessReply, ipc::Handles), messages::ProcessServerError> {
    Err(messages::ProcessServerError::InvalidArgument)
}
