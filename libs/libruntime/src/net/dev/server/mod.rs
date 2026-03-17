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
    net::types::{BufferPool, MacAddress},
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
        buffer_pool: BufferPool,
        link_status_change_callback: impl Fn(bool) + Send + Sync + 'static,
        tx_free_callback: impl Fn(&[u32]) + Send + Sync + 'static,
        rx_arrived_callback: impl Fn(&[iface::RxBufferDescriptor]) + Send + Sync + 'static,
    ) -> Result<Box<Self>, Self::Error>;

    /// Destroy the network device and free any associated resources.
    fn destroy(self);

    /// Get the current link status of the network device
    fn get_link_status(&self) -> Result<bool, Self::Error>;

    /// Get the MAC address of the network device
    fn get_mac_address(&self) -> Result<MacAddress, Self::Error>;

    /// Transmit data on the network device.
    /// Returns the number of descriptors that were accepted for transmission.
    fn tx(&self, descriptors: &[iface::TxBufferDescriptor]) -> Result<usize, Self::Error>;

    /// Add receive buffers to the network device.
    /// Returns the number of buffers that were successfully added.
    fn add_rx_buffers(&self, buffer_indexes: &[u32]) -> Result<usize, Self::Error>;
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
        buffer_pool: BufferPool,
    ) -> Result<Handle, Self::Error> {
        let mut devices = self.devices.write();

        let entry = DeviceEntry::new(name, pci_address, buffer_pool)?;

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

    fn tx(
        &self,
        _sender_id: u64,
        handle: ipc::Handle,
        descriptors: &[iface::TxBufferDescriptor],
    ) -> Result<usize, Self::Error> {
        let entry = self
            .devices
            .read()
            .get(&handle)
            .ok_or_else(|| {
                error!("Invalid handle: {:?}", handle);
                iface::NetDeviceError::InvalidArgument
            })?
            .clone();

        Ok(entry.device()?.tx(descriptors).map_err(Into::into)?)
    }

    fn set_tx_free_port(
        &self,
        _sender_id: u64,
        handle: ipc::Handle,
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

        entry.set_tx_free_port(correlation, port)?;

        Ok(())
    }

    fn add_rx_buffers(
        &self,
        _sender_id: u64,
        handle: ipc::Handle,
        buffer_indexes: &[u32],
    ) -> Result<usize, Self::Error> {
        let entry = self
            .devices
            .read()
            .get(&handle)
            .ok_or_else(|| {
                error!("Invalid handle: {:?}", handle);
                iface::NetDeviceError::InvalidArgument
            })?
            .clone();

        Ok(entry
            .device()?
            .add_rx_buffers(buffer_indexes)
            .map_err(Into::into)?)
    }

    fn set_rx_port(
        &self,
        _sender_id: u64,
        handle: ipc::Handle,
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

        entry.set_rx_port(correlation, port)?;

        Ok(())
    }
}

#[derive(Debug)]
struct DeviceEntry<NetDev: NetDevice> {
    name: String,
    device: RwLock<Option<Box<NetDev>>>, // option set to None when device is destroyed, to prevent further access
    link_status_notifier: Notifier,
    tx_free_notifier: Notifier,
    rx_arrived_notifier: Notifier,
}

impl<NetDev: NetDevice> DeviceEntry<NetDev> {
    /// Creates a new device entry with the given name and device.
    pub fn new(
        name: &str,
        pci_address: PciAddress,
        buffer_pool: BufferPool,
    ) -> Result<Arc<Self>, iface::NetDeviceError> {
        // Create the entry first so that we can pass a reference to the link status change callback to the device
        let entry = Arc::new(Self {
            name: String::from(name),
            device: RwLock::new(None),
            link_status_notifier: Notifier::new(),
            tx_free_notifier: Notifier::new(),
            rx_arrived_notifier: Notifier::new(),
        });

        let link_status_change_callback = {
            let entry = entry.clone();
            move |link_up| {
                entry.link_status_change_callback(link_up);
            }
        };

        let tx_free_callback = {
            let entry = entry.clone();
            move |buffer_indexes: &[u32]| {
                entry.tx_free_callback(buffer_indexes);
            }
        };

        let rx_arrived_callback = {
            let entry = entry.clone();
            move |descriptors: &[iface::RxBufferDescriptor]| {
                entry.rx_arrived_callback(descriptors);
            }
        };

        let device = NetDev::create(
            name,
            pci_address,
            buffer_pool,
            link_status_change_callback,
            tx_free_callback,
            rx_arrived_callback,
        )
        .map_err(Into::into)?;

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

    /// Sets the port for Tx free notifications.
    pub fn set_tx_free_port(
        &self,
        correlation: u64,
        port: Option<kobject::PortSender>,
    ) -> Result<(), iface::NetDeviceError> {
        Notifier::set(&self.tx_free_notifier, &self.name, correlation, port)?;

        Ok(())
    }

    /// Sets the port for Rx arrived notifications.
    pub fn set_rx_port(
        &self,
        correlation: u64,
        port: Option<kobject::PortSender>,
    ) -> Result<(), iface::NetDeviceError> {
        Notifier::set(&self.rx_arrived_notifier, &self.name, correlation, port)?;

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

    /// Callback for when Tx buffers are freed.
    /// This should be called by the device implementation when buffers are freed.
    pub fn tx_free_callback(&self, buffer_indexes: &[u32]) {
        assert!(
            buffer_indexes.len() <= iface::TxFreeNotification::BUFFER_COUNT,
            "Too many buffer indexes for TxFreeNotification"
        );

        Notifier::notify(&self.tx_free_notifier, &self.name, |correlation| {
            let mut buffers = [BufferPool::INVALID_INDEX; iface::TxFreeNotification::BUFFER_COUNT];
            let count = buffer_indexes.len();
            buffers[..count].copy_from_slice(&buffer_indexes[..count]);

            (
                iface::TxFreeNotification {
                    correlation,
                    buffers,
                },
                ipc::KHandles::new(),
            )
        });
    }

    /// Callback for when Rx data arrives.
    /// This should be called by the device implementation when data arrives.
    pub fn rx_arrived_callback(&self, descriptors: &[iface::RxBufferDescriptor]) {
        assert!(
            descriptors.len() <= iface::RxArrivedNotification::DESCRIPTOR_COUNT,
            "Too many descriptors for RxArrivedNotification"
        );

        Notifier::notify(&self.rx_arrived_notifier, &self.name, |correlation| {
            let mut rx_descriptors = [iface::RxBufferDescriptor::default();
                iface::RxArrivedNotification::DESCRIPTOR_COUNT];
            let count = descriptors.len();
            rx_descriptors[..count].copy_from_slice(&descriptors[..count]);

            (
                iface::RxArrivedNotification {
                    correlation,
                    rx_descriptors,
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
