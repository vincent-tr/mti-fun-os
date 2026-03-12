pub mod iface;
mod server;

pub use server::NetDevice;
use server::NetDeviceServer;

use crate::{ipc, kobject};

/// Build a NetDevice IPC server from a NetDevice implementation.
pub fn build_net_device_runner<NetDev: NetDevice>(
    port_name: &'static str,
) -> Result<ipc::Runner, kobject::Error> {
    let server = NetDeviceServer::<NetDev>::new();
    iface::build_ipc_runner(server, port_name)
}
