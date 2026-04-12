use alloc::{boxed::Box, string::String, sync::Arc};
use async_trait::async_trait;

use hashbrown::HashMap;
use libruntime::{
    drivers::pci::PciAddress,
    net::iface::{NetError, NetServer},
    sync::Mutex,
};
use log::error;

use crate::iface::Interface;

/// The main server structure
#[derive(Debug)]
pub struct Server {
    ifaces: Mutex<HashMap<String, Arc<Interface>>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            ifaces: Mutex::new(HashMap::new()),
        }
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
        let mut ifaces = self.ifaces.lock();

        if ifaces.contains_key(name) {
            error!("Interface '{}' already exists", name);
            return Err(NetError::InvalidArgument);
        }

        let iface = Interface::create(name, driver_port_name, pci_address)
            .await
            .map_err(|e| {
                error!("Failed to create interface '{}': {}", name, e);
                NetError::DeviceError
            })?;

        ifaces.insert(String::from(name), iface);

        Ok(())
    }

    async fn destroy_interface(&self, sender_id: u64, name: &str) -> Result<(), Self::Error> {
        let mut ifaces = self.ifaces.lock();

        let Some(iface) = ifaces.remove(name) else {
            error!("Interface '{}' does not exist", name);
            return Err(NetError::InvalidArgument);
        };

        let iface = Arc::try_unwrap(iface).map_err(|iface| {
            ifaces.insert(String::from(name), iface);
            error!("Failed to destroy interface '{}': still in use", name);
            NetError::RuntimeError
        })?;

        iface.destroy().await.map_err(|e| {
            error!("Failed to destroy interface '{}': {}", name, e);
            NetError::DeviceError
        })?;

        Ok(())
    }
}
