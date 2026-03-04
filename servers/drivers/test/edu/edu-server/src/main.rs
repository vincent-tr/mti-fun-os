#![no_std]
#![no_main]

// Device documentation:
// https://www.qemu.org/docs/master/specs/edu.html
//
// Device source code:
// https://gitlab.com/qemu-project/qemu/-/blob/master/hw/misc/edu.c

extern crate libruntime;

use libruntime::drivers::pci::iface::Client as PciClient;
use libruntime::drivers::pci::types::PciAddress;

/// QEMU EDU device vendor and device IDs
const EDU_VENDOR_ID: u16 = 0x1234;
const EDU_DEVICE_ID: u16 = 0x11e8;

/// EDU device register offsets (MMIO BAR0)
#[allow(dead_code)]
mod registers {
    /// Identification register (read-only, should be 0x010000ed)
    pub const ID: usize = 0x00;
    /// Card liveness check (read-write)
    pub const LIVENESS: usize = 0x04;
    /// Factorial computation (read-write)
    pub const FACTORIAL: usize = 0x08;
    /// Status register (read-only)
    pub const STATUS: usize = 0x20;
    /// Interrupt status (read-only)
    pub const IRQ_STATUS: usize = 0x24;
    /// Interrupt raise (write-only)
    pub const IRQ_RAISE: usize = 0x60;
    /// Interrupt acknowledge (write-only)
    pub const IRQ_ACK: usize = 0x64;
    /// DMA source address (read-write)
    pub const DMA_SRC: usize = 0x80;
    /// DMA destination address (read-write)
    pub const DMA_DST: usize = 0x88;
    /// DMA count (read-write)
    pub const DMA_COUNT: usize = 0x90;
    /// DMA command (write-only)
    pub const DMA_CMD: usize = 0x98;
}

/// EDU device status flags
#[allow(dead_code)]
mod status {
    pub const COMPUTING: u32 = 1 << 0;
    pub const IRQ_RAISED: u32 = 1 << 8;
}

/// EDU device DMA commands
#[allow(dead_code)]
mod dma_cmd {
    pub const START: u32 = 1 << 0;
    pub const IRQ: u32 = 1 << 1;
}

struct EduDevice {
    pci_address: PciAddress,
    _bar0_base: u64,
    _bar0_size: u64,
}

impl EduDevice {
    fn new(pci_address: PciAddress) -> Self {
        log::info!(
            "Found EDU device at {}:{}:{}",
            pci_address.bus,
            pci_address.device,
            pci_address.function
        );

        // TODO: Get BAR0 information from PCI configuration space
        // TODO: Map BAR0 MMIO region to virtual memory

        Self {
            pci_address,
            _bar0_base: 0,
            _bar0_size: 0,
        }
    }

    // TODO: Implement MMIO read/write helpers
    // fn read_reg(&self, offset: usize) -> u32 { ... }
    // fn write_reg(&self, offset: usize, value: u32) { ... }

    // TODO: Implement EDU device functionality
    // - Test identification register
    // - Liveness check
    // - Factorial computation
    // - Interrupt handling
    // - DMA operations
}

fn find_edu_device() -> Option<PciAddress> {
    let pci = PciClient::new();

    match pci.list(Some(EDU_VENDOR_ID), Some(EDU_DEVICE_ID), None, None) {
        Ok(devices) => {
            if let Some(device) = devices.first() {
                log::info!(
                    "Found QEMU EDU device: vendor={:#x}, device={:#x}",
                    device.device_id.vendor,
                    device.device_id.device
                );
                Some(device.address)
            } else {
                log::warn!("No EDU device found on PCI bus");
                None
            }
        }
        Err(e) => {
            log::error!("Failed to enumerate PCI devices: {:?}", e);
            None
        }
    }
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    log::info!("edu-server starting...");

    // Find the EDU device on the PCI bus
    let pci_address = match find_edu_device() {
        Some(addr) => addr,
        None => {
            log::error!("EDU device not found, exiting");
            return 1;
        }
    };

    // Initialize the EDU device
    let _edu = EduDevice::new(pci_address);

    log::info!("EDU device initialized successfully");

    // TODO: Implement device operations and service loop
    loop {
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}
