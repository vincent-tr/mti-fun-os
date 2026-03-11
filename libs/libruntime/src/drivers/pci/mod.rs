pub mod iface;
pub mod types;

use alloc::vec::Vec;

pub use iface::{CapabilityInfo, EnableMsiData, PciDeviceInfo, PciServerCallError, PciServerError};
pub use types::*;

use crate::{ipc, kobject};

lazy_static::lazy_static! {
    static ref CLIENT: iface::Client = iface::Client::new();
}

/// Options for listing PCI devices, allowing filtering by vendor ID, device ID, class, and subclass.
#[derive(Debug)]
pub struct ListOptions {
    vendor_id: Option<u16>,
    device_id: Option<u16>,
    class: Option<u8>,
    subclass: Option<u8>,
}

impl ListOptions {
    /// Creates a new `ListOptions` with all fields set to `None`, meaning no filtering.
    pub fn new() -> Self {
        Self {
            vendor_id: None,
            device_id: None,
            class: None,
            subclass: None,
        }
    }

    /// Sets the vendor ID filter for listing PCI devices.
    pub fn with_vendor_id(mut self, vendor_id: u16) -> Self {
        self.vendor_id = Some(vendor_id);
        self
    }

    /// Sets the vendor ID and device ID filters for listing PCI devices.
    pub fn with_device_id(mut self, vendor_id: u16, device_id: u16) -> Self {
        self.vendor_id = Some(vendor_id);
        self.device_id = Some(device_id);
        self
    }

    /// Sets the class filter for listing PCI devices.
    pub fn with_class(mut self, class: u8) -> Self {
        self.class = Some(class);
        self
    }

    /// Sets the class and subclass filters for listing PCI devices.
    pub fn with_subclass(mut self, class: u8, subclass: u8) -> Self {
        self.class = Some(class);
        self.subclass = Some(subclass);
        self
    }
}

/// Lists PCI devices with optional filtering by vendor ID, device ID, class, and subclass.
pub fn list(options: ListOptions) -> Result<Vec<PciDeviceInfo>, PciServerCallError> {
    CLIENT.list(
        options.vendor_id,
        options.device_id,
        options.class,
        options.subclass,
    )
}

pub fn info(address: PciAddress) -> Result<PciDeviceInfo, PciServerCallError> {
    CLIENT.get_by_address(address)
}

#[derive(Debug)]
struct PciHandle {
    handle: ipc::Handle,
}

impl From<ipc::Handle> for PciHandle {
    fn from(handle: ipc::Handle) -> Self {
        Self { handle }
    }
}

impl Drop for PciHandle {
    fn drop(&mut self) {
        CLIENT
            .close(self.handle)
            .expect("Failed to close pci handle");
    }
}

/// Represents an open PCI device, providing methods to interact with it.
#[derive(Debug)]
pub struct PciDevice {
    device: PciHandle,
}

impl PciDevice {
    /// Open a PCI device at the given address.
    pub fn open(address: PciAddress) -> Result<Self, PciServerCallError> {
        let device = PciHandle::from(CLIENT.open(address)?);
        Ok(Self { device })
    }

    /// Get the PCI header for this device.
    pub fn header(&self) -> Result<PciHeader, PciServerCallError> {
        CLIENT.get_header(self.device.handle)
    }

    /// List the capabilities of this device.
    pub fn capabilities(&self) -> Result<Vec<CapabilityInfo>, PciServerCallError> {
        CLIENT.list_capabilities(self.device.handle)
    }

    /// Enable MSI for this device, returning an `Irq` object that can be used to wait for MSI events.
    pub fn enable_msi(&self, irq_info: &kobject::IrqInfo) -> Result<(), PciServerCallError> {
        CLIENT.enable_msi(
            self.device.handle,
            EnableMsiData::Enable {
                address: irq_info.msi_address as usize,
                vector: irq_info.vector,
            },
        )
    }

    /// Disable MSI for this device.
    pub fn disable_msi(&self) -> Result<(), PciServerCallError> {
        CLIENT.enable_msi(self.device.handle, EnableMsiData::Disable)
    }
}
