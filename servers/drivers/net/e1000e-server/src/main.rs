#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

use alloc::sync::Arc;
use libruntime::net::{
    MacAddress,
    dev::{NetDevice, build_net_device_server, iface::NetDeviceError},
};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    // 8086:10d3

    // TODO:
    // netdev interface
    // init flow (PCI probing, device initialization, net stack registration)

    let ipc_server = build_net_device_server::<E1000eDevice>("net.dev.e1000e")
        .expect("failed to build net.dev.e1000e IPC server");

    ipc_server.run()
}

/// Represents an E1000e network device.
#[derive(Debug)]
pub struct E1000eDevice {}

impl NetDevice for E1000eDevice {
    type Error = NetDeviceError;

    fn create(
        name: &str,
        pci_address: libruntime::drivers::pci::PciAddress,
    ) -> Result<Arc<Self>, Self::Error> {
        Ok(Arc::new(Self {}))
    }

    fn destroy(&self) -> Result<(), Self::Error> {
        todo!()
    }

    fn get_link_status(&self) -> Result<bool, Self::Error> {
        todo!()
    }

    fn set_link_status_change_callback(
        &self,
        callback: impl Fn(bool) + Send + 'static,
    ) -> Result<MacAddress, Self::Error> {
        todo!()
    }

    fn get_mac_address(&self) -> Result<MacAddress, Self::Error> {
        todo!()
    }
}
