pub mod iface;
mod server;

pub use server::NetDevice;
use server::NetDeviceServer;

use crate::{ipc, kobject};

/// Build a NetDevice IPC server from a NetDevice implementation.
pub fn build_net_device_server<NetDev: NetDevice>(
    port_name: &'static str,
) -> Result<ipc::Server, kobject::Error> {
    let server = NetDeviceServer::<NetDev>::new();
    iface::build_ipc_server(server, port_name)
}
