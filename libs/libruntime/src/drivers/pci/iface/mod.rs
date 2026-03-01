mod client;
mod info_block;
mod messages;
mod server;

pub use client::{Client, PciServerCallError};
pub use info_block::PciDeviceInfo;
use server::Server;
pub use server::{PciServer, PciServerError};

pub use messages::PORT_NAME;

use crate::{ipc, kobject};

pub fn build_ipc_server<Impl: PciServer + 'static>(
    inner: Impl,
) -> Result<ipc::Server, kobject::Error> {
    let server = Server::new(inner);
    server.build_ipc_server()
}
