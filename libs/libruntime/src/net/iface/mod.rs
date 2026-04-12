mod client;
mod messages;
mod server;

use crate::{ipc, kobject};

pub use client::{Client, NetServerCallError};
pub use messages::NetError;
pub use server::NetServer;
use server::Server;

/// Build an IPC server from a NetServer implementation.
pub fn build_ipc_server<Impl: NetServer + 'static>(
    inner: Impl,
    port_name: &'static str,
) -> Result<ipc::AsyncServer, kobject::Error> {
    let server = Server::new(inner);
    server.build_ipc_server(port_name)
}
