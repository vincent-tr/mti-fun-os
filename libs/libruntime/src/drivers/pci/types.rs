use core::fmt;

/// PCI device address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PciAddress {
    /// The bus number of the PCI address (0-255).
    pub bus: u8,

    /// The device number of the PCI address (0-31).
    pub device: u8,

    /// The function number of the PCI address (0-7).
    pub function: u8,
}

impl fmt::Display for PciAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}.{:01x}",
            self.bus, self.device, self.function
        )
    }
}

/// PCI device ID (vendor ID and device ID)
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PciDeviceId {
    /// The vendor ID of the PCI device (16 bits).
    pub vendor: u16,

    /// The device ID of the PCI device (16 bits).
    pub device: u16,
}

impl fmt::Display for PciDeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vendor, self.device)
    }
}

/// PCI class information
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PciClass {
    /// The class code of the PCI device (8 bits).
    pub class: u8,

    /// The subclass code of the PCI device (8 bits).
    pub subclass: u8,

    /// The programming interface code of the PCI device (8 bits).
    pub prog_if: u8,

    /// The revision ID of the PCI device (8 bits).
    pub revision_id: u8,
}

impl fmt::Display for PciClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}:{:02x}", self.class, self.subclass)
    }
}
