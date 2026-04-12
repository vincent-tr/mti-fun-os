use alloc::boxed::Box;
use async_trait::async_trait;

use libruntime::{
    drivers::pci::PciAddress,
    net::iface::{NetError, NetServer},
};

/// The main server structure
#[derive(Debug)]
pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NetServer for Server {
    type Error = NetError;

    async fn process_terminated(&self, _pid: u64) {}

    async fn create_interface(
        &self,
        sender_id: u64,
        name: &str,
        driver_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<(), Self::Error> {
        Err(NetError::InvalidArgument)
    }

    async fn destroy_interface(&self, sender_id: u64, name: &str) -> Result<(), Self::Error> {
        Err(NetError::InvalidArgument)
    }
}
