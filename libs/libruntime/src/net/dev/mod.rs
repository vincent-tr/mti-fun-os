pub mod iface;
mod server;

use server::NetDeviceServer;
pub use server::{NetDevice, RX_ARRIVED_DESCRIPTOR_COUNT, TX_FREE_BUFFER_COUNT};

use crate::{kobject, service};

/// Build a NetDevice IPC server from a NetDevice implementation.
pub fn setup_net_device_server<NetDev: NetDevice>(
    port_name: &'static str,
    runner: &service::Runner,
) -> Result<(), kobject::Error> {
    let server = NetDeviceServer::<NetDev>::new();
    iface::setup_ipc_server(server, port_name, runner)
}
