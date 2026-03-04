use core::{
    mem, panic,
    sync::atomic::{AtomicBool, Ordering},
};

use libruntime::drivers::pci::{
    iface::PciDeviceInfo,
    types::{
        Bar, InterruptPin, IoBar, MemoryBar, MemoryBarWidth, PciAddress, PciClass, PciDeviceId,
        PciHeader,
    },
};
use log::warn;

use crate::pci::{
    self, CommandRegister, CommonHeader, ConfigurationSpace, GeneralDeviceHeader, Header,
    MemorySpaceBar64, PciToCardBusBridgeHeader, PciToPciBridgeHeader, StatusRegister,
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

        let mut device = Self {
            address,
            device_id,
            class,
            header: None,
            in_use: AtomicBool::new(false),
        };

        if let Some(header) = header.general_device() {
            // Only generic devices can be opened, so we only need to read the header for those.
            let interrupt_line = header.interrupt_line;
            let interrupt_line = if interrupt_line == 0xFF {
                // 0xFF means the device does not use interrupts, so we set it to None in that case.
                None
            } else {
                Some(interrupt_line)
            };

            let interrupt_pin = match header.interrupt_pin {
                0 => None, // 0 means the device does not use interrupts, so we set it to None in that case.
                1 => Some(InterruptPin::PinA),
                2 => Some(InterruptPin::PinB),
                3 => Some(InterruptPin::PinC),
                4 => Some(InterruptPin::PinD),
                _ => panic!("Invalid interrupt pin value: {:#02x}", header.interrupt_pin),
            };

            // Disable the device before reading the BARs, to avoid any potential issues with the device trying to access memory or I/O space while we're reading its configuration.
            device.enable(false, false, false);

            let mut bars: [Option<Bar>; 6] = [None, None, None, None, None, None];

            let mut bar_index = 0;
            while bar_index < 6 {
                let bar = device.read_bar(header, bar_index);

                bars[bar_index] = bar;

                if let Some(bar) = &bar
                    && let Bar::Memory(memory_bar) = bar
                    && memory_bar.width == MemoryBarWidth::Bits64
                {
                    // 64-bit BARs occupy two BAR slots, so we need to skip the next slot after reading a 64-bit BAR.
                    bar_index += 2;
                } else {
                    bar_index += 1;
                }
            }

            // Restore the original state of the device after reading the BARs.
            let cmd_reg = header.common.command;
            device.enable(
                cmd_reg.memory_space_enabled(),
                cmd_reg.io_space_enabled(),
                cmd_reg.bus_master_enabled(),
            );

            device.header = Some(PciHeader {
                subsystem_vendor_id: header.subsystem_vendor_id,
                subsystem_id: header.subsystem_id,
                bars,
                interrupt_line,
                interrupt_pin,
            });
        }

        device
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

    /// Read the specified BAR for the device.
    fn read_bar(&self, header: &GeneralDeviceHeader, index: usize) -> Option<Bar> {
        let bar_by_index = |index: usize| match index {
            0 => (header.bar0, mem::offset_of!(GeneralDeviceHeader, bar0)),
            1 => (header.bar1, mem::offset_of!(GeneralDeviceHeader, bar1)),
            2 => (header.bar2, mem::offset_of!(GeneralDeviceHeader, bar2)),
            3 => (header.bar3, mem::offset_of!(GeneralDeviceHeader, bar3)),
            4 => (header.bar4, mem::offset_of!(GeneralDeviceHeader, bar4)),
            5 => (header.bar5, mem::offset_of!(GeneralDeviceHeader, bar5)),
            _ => panic!("Invalid BAR index: {}", index),
        };

        let (bar, offset) = bar_by_index(index);

        if !bar.is_implemented() {
            return None;
        }

        if bar.is_io() {
            return Some(Bar::Io(self.read_io_bar(&bar, offset)));
        }

        if bar.is_memory() {
            if unsafe { bar.memory_space }.is_64_bit() {
                assert!(
                    index < 5,
                    "64-bit BAR cannot be the last BAR (index 5) because it occupies two BAR slots"
                );

                let (high_bar, _) = bar_by_index(index + 1);
                let bar64 = unsafe { MemorySpaceBar64::from_bars(bar, high_bar) };

                return Some(Bar::Memory(self.read_memory_bar64(&bar64, offset)));
            } else {
                return Some(Bar::Memory(self.read_memory_bar(&bar, offset)));
            }
        }

        None
    }

    /// Read the specified BAR as an I/O BAR.
    fn read_io_bar(&self, bar: &pci::Bar, offset: usize) -> IoBar {
        // Write 1s to the BAR to find out the size of the I/O space it occupies
        unsafe {
            let mut size_checker_bar = *bar;
            size_checker_bar.io_space.set_hightest_address();
            self.write(offset, size_checker_bar.into());
            size_checker_bar = self.read(offset).into();
            let size = size_checker_bar.io_space.read_size();

            // Restore the original BAR value
            self.write(offset, (*bar).into());

            IoBar {
                address: bar.io_space.address() as usize,
                size: size as usize,
            }
        }
    }

    /// Read the specified BAR as a 32-bit memory BAR.
    fn read_memory_bar(&self, bar: &pci::Bar, offset: usize) -> MemoryBar {
        // Write 1s to the BAR to find out the size of the memory space it occupies
        unsafe {
            let mut size_checker_bar = *bar;
            size_checker_bar.memory_space.set_hightest_address();
            self.write(offset, size_checker_bar.into());
            size_checker_bar = self.read(offset).into();

            // Restore the original BAR value
            self.write(offset, (*bar).into());

            MemoryBar {
                address: bar.memory_space.address() as usize,
                size: size_checker_bar.memory_space.read_size() as usize,
                prefetchable: bar.memory_space.prefetchable(),
                width: MemoryBarWidth::Bits32,
            }
        }
    }

    /// Read the specified BAR as a 64-bit memory BAR.
    fn read_memory_bar64(&self, bar: &pci::MemorySpaceBar64, offset: usize) -> MemoryBar {
        // Write 1s to the BAR to find out the size of the memory space it occupies
        let mut size_checker_bar = *bar;
        size_checker_bar.set_hightest_address();
        let (low, high) = size_checker_bar.into();
        self.write(offset, low);
        self.write(offset + mem::size_of::<pci::Bar>(), high);
        size_checker_bar = (
            self.read(offset),
            self.read(offset + mem::size_of::<pci::Bar>()),
        )
            .into();

        // Restore the original BAR value
        let (low, high) = (*bar).into();
        self.write(offset, low);
        self.write(offset + mem::size_of::<pci::Bar>(), high);

        MemoryBar {
            address: bar.address() as usize,
            size: size_checker_bar.read_size() as usize,
            prefetchable: bar.prefetchable(),
            width: MemoryBarWidth::Bits64,
        }
    }

    /// Read from the PCI config space for the device.
    fn read(&self, offset: usize) -> u32 {
        ConfigurationSpace::get().read_u32(self.address, offset)
    }

    /// Write to the PCI config space for the device.
    fn write(&self, offset: usize, value: u32) {
        ConfigurationSpace::get().write_u32(self.address, offset, value);
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

        let mut reg: Reg1 = unsafe { mem::transmute(self.read(0x04)) };
        reg.command.enable_memory_space(memory);
        reg.command.enable_io_space(io);
        reg.command.enable_bus_master(bus_master);
        self.write(0x04, unsafe { mem::transmute(reg) });
    }
}
