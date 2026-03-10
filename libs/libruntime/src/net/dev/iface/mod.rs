mod client;
mod messages;
mod server;

pub use client::{Client, NetDeviceServerCallError};
pub use messages::{LinkStatusChangedNotification, NetDeviceError};
pub use server::{NetDeviceServer, Server};

use crate::{ipc, kobject};

/// Build an IPC server from a NetDeviceServer implementation.
pub fn build_ipc_server<Impl: NetDeviceServer + 'static>(
    inner: Impl,
    port_name: &'static str,
) -> Result<ipc::Server, kobject::Error> {
    let server = Server::new(inner);
    server.build_ipc_server(port_name)
}
