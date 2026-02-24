mod client;
mod messages;
mod server;

pub use client::{Client, TimeServerCallError};
use server::Server;
pub use server::TimeServer;

pub use messages::{PORT_NAME, TimeServerError, Timestamp};

use crate::{ipc, kobject};

pub fn build_ipc_server<Impl: TimeServer + 'static>(
    inner: Impl,
) -> Result<ipc::Server, kobject::Error> {
    let server = Server::new(inner);
    server.build_ipc_server()
}
