use alloc::vec::Vec;
use core::mem;
use libruntime::{
    drivers::pci::types::{PciAddress, PciClass, PciDeviceId},
    kobject,
};

/// PCI configuration space access
#[derive(Debug)]
pub struct ConfigurationSpace {
    address: kobject::PortRange,
    data: kobject::PortRange,
}

impl ConfigurationSpace {
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

    /// Reads a 32-bit value from the PCI configuration space of the specified address and offset.
    fn read_u32(&self, address: PciAddress, offset: u8) -> u32 {
        assert!(
            offset % mem::size_of::<u32>() as u8 == 0,
            "Offset must be aligned to size of u32"
        );

        // https://wiki.osdev.org/PCI#Configuration_Space_Access_Mechanism_#1
        let address = (1 << 31)
            | ((address.bus as u32) << 16)
            | ((address.device as u32) << 11)
            | ((address.function as u32) << 8)
            | (offset as u32); // & 0xFC => unnecessary since we assert that offset is aligned to 4 bytes

        self.address
            .write32(0, address)
            .expect("Failed to write to PCI configuration address port");
        self.data
            .read32(0)
            .expect("Failed to read from PCI configuration space")
    }

    /// Reads a 16-bit value from the PCI configuration space of the specified address and offset.
    fn read_u16(&self, address: PciAddress, offset: u8) -> u16 {
        assert!(
            offset % mem::size_of::<u16>() as u8 == 0,
            "Offset must be aligned to size of u16"
        );

        // Read the 32-bit value containing the desired 16 bits
        let value = self.read_u32(address, offset & 0xFC);
        let shift = (offset % 4) * 8;
        ((value >> shift) & 0xFFFF) as u16
    }

    /// Reads an 8-bit value from the PCI configuration space of the specified address and offset.
    fn read_u8(&self, address: PciAddress, offset: u8) -> u8 {
        // No need to assert alignment for u8, as any offset is valid

        // Read the 32-bit value containing the desired 8 bits
        let value = self.read_u32(address, offset & 0xFC);
        let shift = (offset % 4) * 8;
        ((value >> shift) & 0xFF) as u8
    }

    /// Scans the specified PCI bus for devices and returns a list of found devices.
    pub fn scan_bus(&self, bus: u8) -> Vec<PciAddress> {
        let mut addresses = Vec::new();

        for device in 0..32 {
            if let Some(address) = self.scan_function(bus, device, 0) {
                addresses.push(address);

                // If the multi-function bit (bit 7) is set, there may be additional functions to scan
                if self.get_header_type(address) & 0x80 != 0 {
                    for function in 1..8 {
                        if let Some(address) = self.scan_function(bus, device, function) {
                            addresses.push(address);
                        }
                    }
                }
            }
        }

        addresses
    }

    fn scan_function(&self, bus: u8, device: u8, function: u8) -> Option<PciAddress> {
        let address = PciAddress {
            bus,
            device,
            function,
        };
        let vendor_id = self.read_u16(address, 0x00);

        if vendor_id == 0xFFFF {
            None // No device present
        } else {
            Some(address)
        }
    }

    /// Gets the header type of the specified address.
    fn get_header_type(&self, address: PciAddress) -> u8 {
        self.read_u8(address, 0x0E)
    }

    /// Gets the vendor ID and device ID of the specified address.
    pub fn get_id(&self, address: PciAddress) -> PciDeviceId {
        let id = self.read_u32(address, 0x00);

        PciDeviceId {
            vendor: (id & 0xFFFF) as u16,
            device: ((id >> 16) & 0xFFFF) as u16,
        }
    }

    /// Gets the class code, subclass code, and programming interface of the specified address.
    pub fn get_class(&self, address: PciAddress) -> PciClass {
        let class = self.read_u32(address, 0x08);

        PciClass {
            class: (class >> 24) as u8,
            subclass: ((class >> 16) & 0xFF) as u8,
            prog_if: ((class >> 8) & 0xFF) as u8,
            revision_id: (class & 0xFF) as u8,
        }
    }
}
