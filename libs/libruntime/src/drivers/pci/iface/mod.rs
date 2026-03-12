mod capability_block;
mod client;
mod info_block;
mod messages;
mod server;

pub use capability_block::CapabilityInfo;
pub use client::{Client, PciServerCallError};
pub use info_block::PciDeviceInfo;
use server::Server;
pub use server::{PciServer, PciServerError};

pub use messages::{EnableMsiData, PORT_NAME};

use crate::{kobject, service};

pub fn setup_ipc_server<Impl: PciServer + 'static>(
    inner: Impl,
    runner: &service::Runner,
) -> Result<(), kobject::Error> {
    let server = Server::new(inner);
    server.setup_ipc_server(runner)
}
