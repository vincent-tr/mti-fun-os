mod messages;

mod client;
mod server;

pub use messages::{PORT_NAME, STATE_SIZE, StateServerError};

use crate::{ipc, kobject};

use server::Server;
pub use server::StateServer;

pub use client::{Client, StateServerCallError};

pub fn build_ipc_server<Impl: StateServer + 'static>(
    inner: Impl,
) -> Result<ipc::Server, kobject::Error> {
    let server = Server::new(inner);
    server.build_ipc_server()
}
