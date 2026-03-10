use alloc::{string::String, sync::Arc};
use hashbrown::HashMap;
use log::{debug, error};

use crate::{
    drivers::pci::PciAddress,
    ipc::{self, Handle},
    kobject,
    net::types::MacAddress,
    sync::RwLock,
};

use super::{iface, state::State};

/// Trait representing a network device
pub trait NetDevice: Sync + Send + 'static {
    type Error: Into<iface::NetDeviceError>;

    /// Create a new network device from a PCI address.
    fn create(name: &str, pci_address: PciAddress) -> Result<Arc<Self>, Self::Error>;

    /// Destroy the network device and free any associated resources.
    fn destroy(&self) -> Result<(), Self::Error>;

    /// Get the current link status of the network device
    fn get_link_status(&self) -> Result<bool, Self::Error>;

    /// Set a callback function to be called when the link status changes
    fn set_link_status_change_callback(
        &self,
        callback: impl Fn(bool) + Send + 'static,
    ) -> Result<MacAddress, Self::Error>;

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

        let entry = DeviceEntry::new(
            String::from(name),
            NetDev::create(name, pci_address).map_err(Into::into)?,
        )?;

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

        entry.device.destroy().map_err(Into::into)?;

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

        Ok(entry.device.get_link_status().map_err(Into::into)?)
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

        Ok(entry.device.get_mac_address().map_err(Into::into)?)
    }
}

#[derive(Debug)]
struct DeviceEntry<NetDev: NetDevice> {
    name: String,
    device: Arc<NetDev>,
    link_status_notifier: RwLock<Option<Notifier>>,
}

impl<NetDev: NetDevice> DeviceEntry<NetDev> {
    pub fn new(name: String, device: Arc<NetDev>) -> Result<Arc<Self>, iface::NetDeviceError> {
        let entry = Arc::new(Self {
            name,
            device,
            link_status_notifier: RwLock::new(None),
        });

        entry
            .device
            .set_link_status_change_callback({
                let entry = entry.clone();
                move |link_up| {
                    entry.link_status_change_callback(link_up);
                }
            })
            .map_err(Into::into)?;

        Ok(entry)
    }

    pub fn set_link_status_change_port(
        &self,
        correlation: u64,
        port: Option<kobject::PortSender>,
    ) -> Result<(), iface::NetDeviceError> {
        Notifier::set(&self.link_status_notifier, &self.name, correlation, port)?;

        Ok(())
    }

    pub fn link_status_change_callback(&self, link_up: bool) {
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

#[derive(Debug)]
struct Notifier {
    correlation: u64,
    port: kobject::PortSender,
}

impl Notifier {
    pub fn set(
        notifier: &RwLock<Option<Notifier>>,
        dev_name: &str,
        correlation: u64,
        port: Option<kobject::PortSender>,
    ) -> Result<(), iface::NetDeviceError> {
        let mut notifier = notifier.write();

        // forbid to overwrite existing value
        if notifier.is_some() && port.is_some() {
            error!("Notifier already set for device {}", dev_name);
            return Err(iface::NetDeviceError::InvalidArgument);
        }

        if let Some(port) = port {
            debug!("Registering change notifier for device {}", dev_name);
            *notifier = Some(Notifier { correlation, port });
        } else {
            debug!("Unregistering change notifier for device {}", dev_name);
            *notifier = None;
        }

        Ok(())
    }

    pub fn notify<T: Copy>(
        notifier: &RwLock<Option<Notifier>>,
        dev_name: &str,
        creator: impl Fn(u64) -> (T, ipc::KHandles),
    ) {
        let notifier_access = notifier.read();
        let Some(notifier) = &*notifier_access else {
            return;
        };

        let (data, handles) = creator(notifier.correlation);
        let mut message = unsafe { kobject::Message::new(&data, handles.into()) };

        if let Err(err) = notifier.port.send(&mut message) {
            error!(
                "Failed to send notification for device {}: {:?}",
                dev_name, err
            );
        }
    }
}
