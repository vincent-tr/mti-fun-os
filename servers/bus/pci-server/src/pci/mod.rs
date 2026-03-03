mod configuration_space;
mod layout;

use core::mem;

use alloc::vec::Vec;
pub use configuration_space::ConfigurationSpace;
pub use layout::*;
use libruntime::drivers::pci::types::PciAddress;

/// Scans the specified PCI bus for devices and returns a list of found devices.
pub fn scan_bus(bus: u8) -> Vec<PciAddress> {
    let mut addresses = Vec::new();

    for device in 0..32 {
        if let Some(address) = scan_function(bus, device, 0) {
            addresses.push(address);

            #[repr(C)]
            struct Reg3 {
                cacheline_size: u8,
                latency_timer: u8,
                header_type: HeaderType,
                bist: u8,
            }

            let reg: Reg3 =
                unsafe { mem::transmute(ConfigurationSpace::get().read_u32(address, 0x0C)) };

            if reg.header_type.multi_function() {
                for function in 1..8 {
                    if let Some(address) = scan_function(bus, device, function) {
                        addresses.push(address);
                    }
                }
            }
        }
    }

    addresses
}

fn scan_function(bus: u8, device: u8, function: u8) -> Option<PciAddress> {
    let address = PciAddress {
        bus,
        device,
        function,
    };

    #[repr(C)]
    struct Reg0 {
        vendor_id: u16,
        device_id: u16,
    }

    // Read first register (vendor ID and device ID) to check if a device is present at this address
    let reg: Reg0 = unsafe { mem::transmute(ConfigurationSpace::get().read_u32(address, 0x00)) };
    if reg.vendor_id == 0xFFFF {
        None // No device present
    } else {
        Some(address)
    }
}
