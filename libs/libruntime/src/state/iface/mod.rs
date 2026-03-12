mod messages;

mod client;
mod server;

pub use messages::{PORT_NAME, STATE_SIZE, StateServerError};

use crate::{kobject, service};

use server::Server;
pub use server::StateServer;

pub use client::{Client, StateServerCallError};

pub fn setup_ipc_server<Impl: StateServer + 'static>(
    inner: Impl,
    runner: &service::Runner,
) -> Result<(), kobject::Error> {
    let server = Server::new(inner);
    server.setup_ipc_server(runner)
}
