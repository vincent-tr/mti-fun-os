mod notifier;
mod state;

use core::ops::Deref;

use alloc::{boxed::Box, string::String, sync::Arc};
use hashbrown::HashMap;
use log::error;

use crate::{
    drivers::pci::PciAddress,
    ipc::{self, Handle},
    kobject,
    net::types::MacAddress,
    sync::{RwLock, RwLockReadGuard},
};

use super::iface;
use notifier::Notifier;
use state::State;

/// Trait representing a network device
pub trait NetDevice: Sync + Send + 'static {
    type Error: Into<iface::NetDeviceError>;

    /// Create a new network device from a PCI address.
    fn create(
        name: &str,
        pci_address: PciAddress,
        link_status_change_callback: impl Fn(bool) + Send + Sync + 'static,
    ) -> Result<Box<Self>, Self::Error>;

    /// Destroy the network device and free any associated resources.
    fn destroy(self);

    /// Get the current link status of the network device
    fn get_link_status(&self) -> Result<bool, Self::Error>;

    /// Get the MAC address of the network device
    fn get_mac_address(&self) -> Result<MacAddress, Self::Error>;

    // TODO: buffer recv/send
}

/// Helper implementation of a net device server for a given NetDevice implementation.
#[derive(Debug)]
pub struct NetDeviceServer<NetDev: NetDevice> {
    devices: RwLock<HashMap<Handle, Arc<DeviceEntry<NetDev>>>>,
}

impl<NetDev: NetDevice> NetDeviceServer<NetDev> {
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(HashMap::new()),
        }
    }

    fn new_handle() -> Handle {
        State::get().handle_generator().generate()
    }
}

impl<NetDev: NetDevice> iface::NetDeviceServer for NetDeviceServer<NetDev> {
    type Error = iface::NetDeviceError;

    fn create(
        &self,
        _sender_id: u64,
        name: &str,
        pci_address: PciAddress,
    ) -> Result<Handle, Self::Error> {
        let mut devices = self.devices.write();

        let entry = DeviceEntry::new(name, pci_address)?;

        let handle = Self::new_handle();
        devices.insert(handle, entry);

        Ok(handle)
    }

    fn destroy(&self, _sender_id: u64, handle: Handle) -> Result<(), Self::Error> {
        let mut devices = self.devices.write();

        let entry = devices.get(&handle).ok_or_else(|| {
            error!("Invalid handle: {:?}", handle);
            iface::NetDeviceError::InvalidArgument
        })?;

        entry.destroy().map_err(Into::into)?;

        devices.remove(&handle);

        Ok(())
    }

    fn get_link_status(&self, _sender_id: u64, handle: Handle) -> Result<bool, Self::Error> {
        let entry = self
            .devices
            .read()
            .get(&handle)
            .ok_or_else(|| {
                error!("Invalid handle: {:?}", handle);
                iface::NetDeviceError::InvalidArgument
            })?
            .clone();

        Ok(entry.device()?.get_link_status().map_err(Into::into)?)
    }

    fn set_link_status_change_port(
        &self,
        _sender_id: u64,
        handle: Handle,
        port: Option<kobject::PortSender>,
        correlation: u64,
    ) -> Result<(), Self::Error> {
        let entry = self
            .devices
            .read()
            .get(&handle)
            .ok_or_else(|| {
                error!("Invalid handle: {:?}", handle);
                iface::NetDeviceError::InvalidArgument
            })?
            .clone();

        entry.set_link_status_change_port(correlation, port)?;

        Ok(())
    }

    fn get_mac_address(&self, _sender_id: u64, handle: Handle) -> Result<MacAddress, Self::Error> {
        let entry = self
            .devices
            .read()
            .get(&handle)
            .ok_or_else(|| {
                error!("Invalid handle: {:?}", handle);
                iface::NetDeviceError::InvalidArgument
            })?
            .clone();

        Ok(entry.device()?.get_mac_address().map_err(Into::into)?)
    }
}

#[derive(Debug)]
struct DeviceEntry<NetDev: NetDevice> {
    name: String,
    device: RwLock<Option<Box<NetDev>>>, // option set to None when device is destroyed, to prevent further access
    link_status_notifier: Notifier,
}

impl<NetDev: NetDevice> DeviceEntry<NetDev> {
    /// Creates a new device entry with the given name and device.
    pub fn new(name: &str, pci_address: PciAddress) -> Result<Arc<Self>, iface::NetDeviceError> {
        // Create the entry first so that we can pass a reference to the link status change callback to the device
        let entry = Arc::new(Self {
            name: String::from(name),
            device: RwLock::new(None),
            link_status_notifier: Notifier::new(),
        });

        let link_status_change_callback = {
            let entry = entry.clone();
            move |link_up| {
                entry.link_status_change_callback(link_up);
            }
        };

        let device =
            NetDev::create(name, pci_address, link_status_change_callback).map_err(Into::into)?;

        *entry.device.write() = Some(device);

        Ok(entry)
    }

    /// Destroys the device associated with this entry and prevents further access to it.
    pub fn destroy(&self) -> Result<(), iface::NetDeviceError> {
        let device = self.device.write().take().ok_or_else(|| {
            error!("Device already destroyed for entry {}", self.name);
            iface::NetDeviceError::InvalidArgument
        })?;

        device.destroy();

        Ok(())
    }

    /// Get access to the device, ensuring it is not accessed after being destroyed.
    pub fn device(&self) -> Result<DeviceAccess<'_, NetDev>, iface::NetDeviceError> {
        let access = self.device.read();

        if access.is_none() {
            error!("Device already destroyed {}", self.name);
            return Err(iface::NetDeviceError::InvalidArgument);
        }

        Ok(DeviceAccess { access })
    }

    /// Sets the port for link status change notifications.
    pub fn set_link_status_change_port(
        &self,
        correlation: u64,
        port: Option<kobject::PortSender>,
    ) -> Result<(), iface::NetDeviceError> {
        Notifier::set(&self.link_status_notifier, &self.name, correlation, port)?;

        Ok(())
    }

    fn link_status_change_callback(&self, link_up: bool) {
        Notifier::notify(&self.link_status_notifier, &self.name, |correlation| {
            (
                iface::LinkStatusChangedNotification {
                    correlation,
                    link_up,
                },
                ipc::KHandles::new(),
            )
        });
    }
}

/// Helper struct to provide access to the device while ensuring it is not accessed after being destroyed.
struct DeviceAccess<'a, NetDev: NetDevice> {
    access: RwLockReadGuard<'a, Option<Box<NetDev>>>,
}

impl<NetDev: NetDevice> Deref for DeviceAccess<'_, NetDev> {
    type Target = NetDev;

    fn deref(&self) -> &NetDev {
        self.access.as_ref().expect("device should be set")
    }
}
