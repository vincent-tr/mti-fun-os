#![no_std]
#![no_main]

extern crate alloc;
extern crate libruntime;

mod pci;

use log::info;
use pci::{PciClass, PciConfig, PciDevice};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    info!("PCI server started");

    // https://wiki.osdev.org/PCI

    let pci = PciConfig::open();

    let devices = pci.scan_bus(0);
    for device in devices {
        let id = pci.get_id(device);
        let class = pci.get_class(device);

        info!(
            "Found PCI device: bus {}, device {}, function {}, id {}, class {:?}",
            device.bus(),
            device.device(),
            device.function(),
            id,
            class
        );
    }

    0
}
