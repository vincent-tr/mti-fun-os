mod client;
mod messages;
mod server;

pub use client::{Client, NetDeviceServerCallError};
pub use messages::{LinkStatusChangedNotification, NetDeviceError};
pub use server::{NetDeviceServer, Server};

use crate::{kobject, service};

/// Build an IPC server from a NetDeviceServer implementation.
pub fn setup_ipc_server<Impl: NetDeviceServer + 'static>(
    inner: Impl,
    port_name: &'static str,
    runner: &service::Runner,
) -> Result<(), kobject::Error> {
    let server = Server::new(inner);
    server.setup_ipc_server(port_name, runner)
}
