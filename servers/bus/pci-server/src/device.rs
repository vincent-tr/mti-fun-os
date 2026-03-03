use core::{
    mem, panic,
    sync::atomic::{AtomicBool, Ordering},
};

use libruntime::drivers::pci::{
    iface::PciDeviceInfo,
    types::{Bar, InterruptPin, PciAddress, PciClass, PciDeviceId, PciHeader},
};
use log::warn;

use crate::pci::{
    CommandRegister, CommonHeader, ConfigurationSpace, GeneralDeviceHeader, Header,
    PciToCardBusBridgeHeader, PciToPciBridgeHeader, StatusRegister,
};

#[derive(Debug)]
pub struct Device {
    /// The address of the PCI device, which uniquely identifies it on the system.
    address: PciAddress,

    /// The vendor ID and device ID of the PCI device, which can be used to identify the type of device.
    device_id: PciDeviceId,

    /// The class code and subclass code of the PCI device, which can be used to further classify the type of device.
    class: PciClass,

    /// The PCI header for the device, which contains information about the device's capabilities and resources.
    ///
    /// This is only filled for generic devices, not for bridge devices
    /// Only generic devices can be opened, so we only need the header for those.
    header: Option<PciHeader>,

    /// Indicate whether the device is currently in use or not.
    ///
    /// This can be used to prevent multiple processes from trying to access the same device at the same time.
    in_use: AtomicBool,
}

impl Device {
    /// Creates a new `PciDevice` with the given address, device ID, and class.
    pub fn new(address: PciAddress) -> Self {
        let header = Self::get_header(address);
        let common_header = header.common();

        let device_id = PciDeviceId {
            vendor: common_header.vendor_id,
            device: common_header.device_id,
        };

        let class = PciClass {
            class: common_header.class_code,
            subclass: common_header.subclass,
            prog_if: common_header.prog_if,
            revision_id: common_header.revision_id,
        };

        let header = header.general_device().map(|general_device_header| {
            let interrupt_line = general_device_header.interrupt_line;
            let interrupt_line = if interrupt_line == 0xFF {
                // 0xFF means the device does not use interrupts, so we set it to None in that case.
                None
            } else {
                Some(interrupt_line)
            };

            let interrupt_pin = match general_device_header.interrupt_pin {
                0 => None, // 0 means the device does not use interrupts, so we set it to None in that case.
                1 => Some(InterruptPin::PinA),
                2 => Some(InterruptPin::PinB),
                3 => Some(InterruptPin::PinC),
                4 => Some(InterruptPin::PinD),
                _ => panic!(
                    "Invalid interrupt pin value: {:#02x}",
                    general_device_header.interrupt_pin
                ),
            };

            // TODO
            let bars: [Option<Bar>; 6] = [None, None, None, None, None, None];

            PciHeader {
                subsystem_vendor_id: general_device_header.subsystem_vendor_id,
                subsystem_id: general_device_header.subsystem_id,
                bars,
                interrupt_line,
                interrupt_pin,
            }
        });

        Self {
            address,
            device_id,
            class,
            header,
            in_use: AtomicBool::new(false),
        }
    }

    /// Gets the PCI header for the device at the specified address.
    fn get_header(address: PciAddress) -> Header {
        let mut common = CommonHeader::default();
        ConfigurationSpace::get().read_data::<CommonHeader>(address, 0x00, &mut common, None);
        let header_type = common.header_type.r#type();

        // Read the rest of the header based on the header type
        match header_type {
            0x00 => {
                let mut general_device = GeneralDeviceHeader {
                    common,
                    ..Default::default()
                };

                ConfigurationSpace::get().read_data::<GeneralDeviceHeader>(
                    address,
                    0x00,
                    &mut general_device,
                    Some(mem::size_of::<CommonHeader>()..mem::size_of::<GeneralDeviceHeader>()),
                );
                Header { general_device }
            }
            0x01 => {
                let mut pci_bridge = PciToPciBridgeHeader {
                    common,
                    ..Default::default()
                };

                ConfigurationSpace::get().read_data::<PciToPciBridgeHeader>(
                    address,
                    0x00,
                    &mut pci_bridge,
                    Some(mem::size_of::<CommonHeader>()..mem::size_of::<PciToPciBridgeHeader>()),
                );
                Header { pci_bridge }
            }
            0x02 => {
                let mut cardbus_bridge = PciToCardBusBridgeHeader {
                    common,
                    ..Default::default()
                };

                ConfigurationSpace::get().read_data::<PciToCardBusBridgeHeader>(
                    address,
                    0x00,
                    &mut cardbus_bridge,
                    Some(
                        mem::size_of::<CommonHeader>()..mem::size_of::<PciToCardBusBridgeHeader>(),
                    ),
                );
                Header { cardbus_bridge }
            }
            _ => {
                warn!(
                    "Unknown PCI header type {:#02x} for device at bus {}, device {}, function {}",
                    header_type, address.bus, address.device, address.function
                );

                Header { common }
            }
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
        if self.header.is_none() {
            // Only generic devices can be opened, not bridge devices.
            return false;
        }

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

    /// Returns the PCI header for the device, if it is a generic device.
    pub fn header(&self) -> Option<PciHeader> {
        self.header
    }

    /// Enable or disable memory, I/O, and bus mastering for the device.
    pub fn enable(&self, memory: bool, io: bool, bus_master: bool) {
        #[repr(C)]
        struct Reg1 {
            command: CommandRegister,
            status: StatusRegister,
        }

        let mut reg: Reg1 = unsafe { mem::transmute(self.read_config(0x04)) };
        reg.command.enable_memory_space(memory);
        reg.command.enable_io_space(io);
        reg.command.enable_bus_master(bus_master);
        self.write_config(0x04, unsafe { mem::transmute(reg) });
    }

    /// Read from the PCI config space for the device.
    pub fn read_config(&self, offset: usize) -> u32 {
        ConfigurationSpace::get().read_u32(self.address, offset)
    }

    /// Write to the PCI config space for the device.
    pub fn write_config(&self, offset: usize, value: u32) {
        ConfigurationSpace::get().write_u32(self.address, offset, value);
    }
}
