mod client;
mod messages;
mod server;

pub use client::{Client, TimeServerCallError};
use server::Server;
pub use server::TimeServer;

pub use messages::{PORT_NAME, TimeServerError, Timestamp};

use crate::{kobject, service};

pub fn setup_ipc_server<Impl: TimeServer + 'static>(
    inner: Impl,
    runner: &service::Runner,
) -> Result<(), kobject::Error> {
    let server = Server::new(inner);
    server.setup_ipc_server(runner)
}
