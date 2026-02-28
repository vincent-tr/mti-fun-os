mod client;
mod messages;
mod server;

use crate::{ipc, kobject};

// Reuse directory entry types from VFS
pub use crate::file::vfs::iface::{DentriesBlock, DirectoryEntry};

pub use client::{Client, FsServerCallError};
pub use messages::FsServerError;

pub use server::FileSystem;
use server::Server;

pub fn build_ipc_server<Impl: FileSystem + 'static>(
    inner: Impl,
    port_name: &'static str,
) -> Result<ipc::AsyncServer, kobject::Error> {
    let server = Server::new(inner);
    server.build_ipc_server(port_name)
}
