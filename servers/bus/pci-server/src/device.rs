use core::sync::atomic::{AtomicBool, Ordering};

use libruntime::drivers::pci::{
    iface::PciDeviceInfo,
    types::{PciAddress, PciClass, PciDeviceId},
};

#[derive(Debug)]
pub struct Device {
    /// The address of the PCI device, which uniquely identifies it on the system.
    address: PciAddress,

    /// The vendor ID and device ID of the PCI device, which can be used to identify the type of device.
    device_id: PciDeviceId,

    /// The class code and subclass code of the PCI device, which can be used to further classify the type of device.
    class: PciClass,

    /// Indicate whether the device is currently in use or not.
    ///
    /// This can be used to prevent multiple processes from trying to access the same device at the same time.
    in_use: AtomicBool,
}

impl Device {
    /// Creates a new `PciDevice` with the given address, device ID, and class.
    pub fn new(address: PciAddress, device_id: PciDeviceId, class: PciClass) -> Self {
        Self {
            address,
            device_id,
            class,
            in_use: AtomicBool::new(false),
        }
    }

    /// Returns the address of the PCI device.
    pub fn address(&self) -> PciAddress {
        self.address
    }

    /// Returns the device ID of the PCI device.
    pub fn device_id(&self) -> PciDeviceId {
        self.device_id
    }

    /// Returns the class of the PCI device.
    pub fn class(&self) -> PciClass {
        self.class
    }

    /// Returns a `PciDeviceInfo` struct containing the device's information.
    pub fn info(&self) -> PciDeviceInfo {
        PciDeviceInfo {
            address: self.address,
            device_id: self.device_id,
            class: self.class,
        }
    }

    /// Try to mark the device as in use.
    ///
    /// Returns `true` if the device was successfully marked as in use, or `false` if it was already in use.
    pub fn try_open(&self) -> bool {
        !self.in_use.swap(true, Ordering::SeqCst)
    }

    /// Marks the device as closed.
    pub fn closed(&self) {
        self.in_use.store(false, Ordering::SeqCst);
    }

    /// Returns whether the device is currently in use.
    #[allow(dead_code)]
    pub fn is_in_use(&self) -> bool {
        self.in_use.load(Ordering::SeqCst)
    }
}
