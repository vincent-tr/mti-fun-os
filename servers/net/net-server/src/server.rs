use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use async_trait::async_trait;

use hashbrown::HashMap;
use libruntime::{
    drivers::pci::PciAddress,
    net::{
        iface::{NetServer, NetServerError, Route},
        types::{IpAddress, IpPrefix},
    },
    sync::Mutex,
};
use log::error;

use crate::{
    iface::{Interface, IpConfiguration},
    proto::GlobalProtocols,
};

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
    type Error = NetServerError;

    async fn process_terminated(&self, _pid: u64) {}

    async fn create_interface(
        &self,
        _sender_id: u64,
        name: &str,
        driver_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<(), Self::Error> {
        let mut ifaces = self.ifaces.lock();

        if ifaces.contains_key(name) {
            error!("Interface '{}' already exists", name);
            return Err(NetServerError::InvalidArgument);
        }

        let iface = Interface::create(name, driver_port_name, pci_address).await?;

        ifaces.insert(String::from(name), iface);

        Ok(())
    }

    async fn destroy_interface(&self, _sender_id: u64, name: &str) -> Result<(), Self::Error> {
        let mut ifaces = self.ifaces.lock();

        let Some(iface) = ifaces.remove(name) else {
            error!("Interface '{}' does not exist", name);
            return Err(NetServerError::InvalidArgument);
        };

        GlobalProtocols::instance().ip().routes_remove_iface(&iface);

        // Note: we are inconsistent on failure here: we failed to delete the iface, but we cannot keep it alive.
        iface.destroy().await?;

        Ok(())
    }

    async fn set_route(
        &self,
        _sender_id: u64,
        prefix: IpPrefix,
        gateway: Option<IpAddress>,
        iface: &str,
        metric: usize,
    ) -> Result<(), Self::Error> {
        let Some(iface) = self.ifaces.lock().get(iface).cloned() else {
            error!("Interface '{}' does not exist", iface);
            return Err(NetServerError::InvalidArgument);
        };

        GlobalProtocols::instance()
            .ip()
            .route_set(prefix, iface, gateway, metric);

        Ok(())
    }

    async fn remove_route(
        &self,
        _sender_id: u64,
        prefix: IpPrefix,
        iface: &str,
    ) -> Result<(), Self::Error> {
        let Some(iface) = self.ifaces.lock().get(iface).cloned() else {
            error!("Interface '{}' does not exist", iface);
            return Err(NetServerError::InvalidArgument);
        };

        GlobalProtocols::instance()
            .ip()
            .route_remove(prefix, &iface);

        Ok(())
    }

    async fn list_routes(&self, _sender_id: u64) -> Result<Vec<Route>, Self::Error> {
        let routes = GlobalProtocols::instance().ip().routes_list();

        Ok(routes)
    }
}
