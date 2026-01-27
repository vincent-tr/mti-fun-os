#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;
extern crate libruntime;

use log::{error, info};

const MESSAGE_VERSION: u16 = 1;

use libruntime::ipc;

#[no_mangle]
pub fn main() {
    let server = ipc::ServerBuilder::new("process-server", MESSAGE_VERSION)
        .with_handler(MessageType::CreateProcess, create_process)
        .build()
        .expect("failed to build process-server IPC server");

    info!("process-server started");

    server.run();
}

fn create_process(
    query: CreateProcessQueryParameters,
    handles: ipc::Handles,
) -> Result<(CreateProcessReply, ipc::Handles), ProcessServerError> {
    Err(ProcessServerError::InvalidArgument)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
enum MessageType {
    CreateProcess = 1,
    GetStartupInfo = 3,
    UpdateProcessName = 2,
    UpdateEnv = 4,
    SetExitCode = 5,
}

impl From<MessageType> for u16 {
    fn from(value: MessageType) -> Self {
        value as u16
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CreateProcessQueryParameters {
    name: Buffer,
    // Handle[1]: name mem_obj
    // Handle[2]: binary mem_obj - process-server will take ownership - data must start at offset 0
    // Handle[3]: env mem_obj - process-server will take ownership - data must start at offset 0
    // Handle[4]: args mem_obj - process-server will take ownership - data must start at offset 0
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Buffer {
    // will take a handle slot at mem_obj
    offset: usize,
    size: usize,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CreateProcessReply {
    // Handle[0]: create process handle, must close when done
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum ProcessServerError {
    InvalidArgument = 1,
}
