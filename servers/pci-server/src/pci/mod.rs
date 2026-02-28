mod device;

use alloc::vec::Vec;
use core::mem;
use libruntime::kobject;

pub use device::{
    BridgeSubclass, DisplaySubclass, MassStorageSubclass, NetworkSubclass, PciClass, PciDevice,
    PciDeviceId, SerialBusSubclass,
};

/// PCI configuration space access
#[derive(Debug)]
pub struct PciConfig {
    address: kobject::PortRange,
    data: kobject::PortRange,
}

impl PciConfig {
    /// Opens the PCI configuration space access ports.
    pub fn open() -> Self {
        const CONFIG_ADDRESS: u16 = 0xCF8;
        const CONFIG_DATA: u16 = 0xCFC;

        let address = kobject::PortRange::open(
            CONFIG_ADDRESS,
            1,
            kobject::PortAccess::READ | kobject::PortAccess::WRITE,
        )
        .expect("Failed to open CONFIG_ADDRESS port range");
        let data = kobject::PortRange::open(
            CONFIG_DATA,
            4,
            kobject::PortAccess::READ | kobject::PortAccess::WRITE,
        )
        .expect("Failed to open CONFIG_DATA port range");

        Self { address, data }
    }

    /// Reads a 32-bit value from the PCI configuration space of the specified device and offset.
    fn read_u32(&self, device: PciDevice, offset: u8) -> u32 {
        assert!(
            offset % mem::size_of::<u32>() as u8 == 0,
            "Offset must be aligned to size of u32"
        );

        // https://wiki.osdev.org/PCI#Configuration_Space_Access_Mechanism_#1
        let address = (1 << 31)
            | ((device.bus() as u32) << 16)
            | ((device.device() as u32) << 11)
            | ((device.function() as u32) << 8)
            | (offset as u32); // & 0xFC => unnecessary since we assert that offset is aligned to 4 bytes

        self.address
            .write32(0, address)
            .expect("Failed to write to PCI configuration address port");
        self.data
            .read32(0)
            .expect("Failed to read from PCI configuration space")
    }

    /// Reads a 16-bit value from the PCI configuration space of the specified device and offset.
    fn read_u16(&self, device: PciDevice, offset: u8) -> u16 {
        assert!(
            offset % mem::size_of::<u16>() as u8 == 0,
            "Offset must be aligned to size of u16"
        );

        // Read the 32-bit value containing the desired 16 bits
        let value = self.read_u32(device, offset & 0xFC);
        let shift = (offset % 4) * 8;
        ((value >> shift) & 0xFFFF) as u16
    }

    /// Reads an 8-bit value from the PCI configuration space of the specified device and offset.
    fn read_u8(&self, device: PciDevice, offset: u8) -> u8 {
        // No need to assert alignment for u8, as any offset is valid

        // Read the 32-bit value containing the desired 8 bits
        let value = self.read_u32(device, offset & 0xFC);
        let shift = (offset % 4) * 8;
        ((value >> shift) & 0xFF) as u8
    }

    /// Scans the specified PCI bus for devices and returns a list of found devices.
    pub fn scan_bus(&self, bus: u8) -> Vec<PciDevice> {
        let mut devices = Vec::new();

        for device in 0..32 {
            if let Some(pci_device) = self.scan_function(bus, device, 0) {
                devices.push(pci_device);

                // If the multi-function bit (bit 7) is set, there may be additional functions to scan
                if self.get_header_type(pci_device) & 0x80 != 0 {
                    for function in 1..8 {
                        if let Some(pci_device) = self.scan_function(bus, device, function) {
                            devices.push(pci_device);
                        }
                    }
                }
            }
        }

        devices
    }

    fn scan_function(&self, bus: u8, device: u8, function: u8) -> Option<PciDevice> {
        let pci_device = PciDevice::new(bus, device, function);
        let vendor_id = self.read_u16(pci_device, 0x00);

        if vendor_id == 0xFFFF {
            None // No device present
        } else {
            Some(pci_device)
        }
    }

    /// Gets the header type of the specified PCI device.
    fn get_header_type(&self, device: PciDevice) -> u8 {
        self.read_u8(device, 0x0E)
    }

    /// Gets the vendor ID and device ID of the specified PCI device.
    pub fn get_id(&self, device: PciDevice) -> PciDeviceId {
        let id = self.read_u32(device, 0x00);
        let vendor_id = (id & 0xFFFF) as u16;
        let device_id = ((id >> 16) & 0xFFFF) as u16;

        PciDeviceId::new(vendor_id, device_id)
    }

    /// Gets the class code, subclass code, and programming interface of the specified PCI device.
    pub fn get_class(&self, device: PciDevice) -> PciClass {
        let class = self.read_u32(device, 0x08);
        let class_code = (class >> 24) as u8;
        let subclass_code = ((class >> 16) & 0xFF) as u8;
        // let prog_if = ((class >> 8) & 0xFF) as u8;
        // let revision_id = (class & 0xFF) as u8;

        PciClass::parse(class_code, subclass_code)
    }
}
