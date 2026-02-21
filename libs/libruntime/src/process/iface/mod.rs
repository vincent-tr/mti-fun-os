mod client;
mod kvblock;
mod messages;
mod plblock;
mod server;
mod symblock;

use alloc::string::String;
pub use kvblock::KVBlock;
pub use plblock::ProcessInfo;
use plblock::ProcessListBlock;
pub use symblock::SymBlock;

pub use client::{Client, ProcessServerCallError};
pub use server::ProcessServer;
use server::Server;

pub use messages::{
    EXIT_CODE_KILLED, EXIT_CODE_SUCCESS, EXIT_CODE_UNSET, PORT_NAME, ProcessServerError,
    ProcessStatus, ProcessTerminatedNotification,
};

use crate::{ipc, kobject};

/// Process startup information.
#[derive(Debug)]
pub struct StartupInfo {
    /// Name of the process
    pub name: String,

    /// Environment variables of the process
    pub env: KVBlock,

    /// Arguments of the process
    pub args: KVBlock,

    /// Symbol information for the process, used for debugging.
    pub symbols: SymBlock,
}

pub fn build_ipc_server<Impl: ProcessServer + 'static>(
    inner: Impl,
) -> Result<ipc::Server, kobject::Error> {
    let server = Server::new(inner);
    server.build_ipc_server()
}
