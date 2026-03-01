use core::fmt;

/// PCI device information
#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    bus: u8,
    device: u8,
    function: u8,
}

impl PciDevice {
    /// Create a new PCI device with the given bus, device, and function numbers
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device,
            function,
        }
    }

    /// Get the bus number of the PCI device
    pub fn bus(&self) -> u8 {
        self.bus
    }

    /// Get the device number of the PCI device
    pub fn device(&self) -> u8 {
        self.device
    }

    /// Get the function number of the PCI device
    pub fn function(&self) -> u8 {
        self.function
    }
}

/// PCI device ID (vendor ID and device ID)
#[derive(Debug, Copy, Clone)]
pub struct PciDeviceId {
    vendor_id: u16,
    device_id: u16,
}

impl PciDeviceId {
    /// Create a new PCI device ID with the given vendor ID and device ID
    pub fn new(vendor_id: u16, device_id: u16) -> Self {
        Self {
            vendor_id,
            device_id,
        }
    }

    /// Get the vendor ID of the PCI device
    pub fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    /// Get the device ID of the PCI device
    pub fn device_id(&self) -> u16 {
        self.device_id
    }
}

impl fmt::Display for PciDeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vendor_id, self.device_id)
    }
}

/// PCI class and subclass information
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum PciClass {
    MassStorage(MassStorageSubclass),
    Network(NetworkSubclass),
    Display(DisplaySubclass),
    Bridge(BridgeSubclass),
    SerialBus(SerialBusSubclass),
    Unknown { class: u8, subclass: u8 },
}

/// PCI Mass Storage subclasses
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MassStorageSubclass {
    Sata,
    Nvme,
    Ide,
    Other(u8),
}

/// PCI Network subclasses
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum NetworkSubclass {
    Ethernet,
    Other(u8),
}

/// PCI Display subclasses
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum DisplaySubclass {
    Vga,
    ThreeD,
    Other(u8),
}

/// PCI Bridge subclasses
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum BridgeSubclass {
    Host,
    PciToPci,
    Isa,
    Other(u8),
}

/// PCI Serial Bus subclasses
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SerialBusSubclass {
    Usb,
    Smbus,
    Other(u8),
}

impl PciClass {
    /// Parses the PCI class and subclass codes into a `PciClass` enum.
    pub fn parse(class: u8, subclass: u8) -> Self {
        match class {
            0x01 => {
                let sub = match subclass {
                    0x01 => MassStorageSubclass::Ide,
                    0x06 => MassStorageSubclass::Sata,
                    0x08 => MassStorageSubclass::Nvme,
                    other => MassStorageSubclass::Other(other),
                };
                PciClass::MassStorage(sub)
            }

            0x02 => {
                let sub = match subclass {
                    0x00 => NetworkSubclass::Ethernet,
                    other => NetworkSubclass::Other(other),
                };
                PciClass::Network(sub)
            }

            0x06 => {
                let sub = match subclass {
                    0x00 => BridgeSubclass::Host,
                    0x01 => BridgeSubclass::Isa,
                    0x04 => BridgeSubclass::PciToPci,
                    other => BridgeSubclass::Other(other),
                };
                PciClass::Bridge(sub)
            }

            0x0C => {
                let sub = match subclass {
                    0x03 => SerialBusSubclass::Usb,
                    0x05 => SerialBusSubclass::Smbus,
                    other => SerialBusSubclass::Other(other),
                };
                PciClass::SerialBus(sub)
            }

            other => PciClass::Unknown {
                class: other,
                subclass,
            },
        }
    }
}
