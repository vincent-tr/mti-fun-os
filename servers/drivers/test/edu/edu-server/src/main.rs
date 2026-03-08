#![no_std]
#![no_main]

// Device documentation:
// https://www.qemu.org/docs/master/specs/edu.html
//
// Device source code:
// https://gitlab.com/qemu-project/qemu/-/blob/master/hw/misc/edu.c

extern crate libruntime;

use core::{mem, ptr};

use libruntime::{drivers::pci, kobject};
use log::{error, info};

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
    _device: pci::PciDevice,
    mapping: kobject::Mapping<'static>,
}

impl EduDevice {
    pub fn new(address: pci::PciAddress) -> Self {
        info!("Opening EDU device at {}", address);

        let device = pci::PciDevice::open(address).expect("Failed to open PCI device");
        let header = device.header().expect("Failed to get PCI header");

        let bar0 = header.bars[0].expect("BAR0 not found");
        let pci::Bar::Memory(bar0) = bar0 else {
            panic!("BAR0 is not a memory-mapped region");
        };

        info!(
            "BAR0 MMIO region: address=0x{:x}, size=0x{:x}",
            bar0.address, bar0.size
        );

        let iomem =
            unsafe { kobject::MemoryObject::open_iomem(bar0.address, bar0.size, false, true) }
                .expect("Failed to map BAR0 MMIO region");

        let mapping = kobject::Process::current()
            .map_mem(
                None,
                bar0.size,
                kobject::Permissions::READ | kobject::Permissions::WRITE,
                &iomem,
                0,
            )
            .expect("Failed to map BAR0 MMIO region to virtual memory");

        let irq = kobject::Irq::create().expect("Failed to create IRQ object");
        device
            .enable_msi(&irq.info().expect("Failed to get IRQ info"))
            .expect("Failed to enable MSI");

        Self {
            _device: device,
            mapping,
        }
    }

    fn read_reg32(&self, offset: usize) -> u32 {
        assert!(offset + mem::size_of::<u32>() <= self.mapping.len());
        assert!(offset % mem::size_of::<u32>() == 0);

        unsafe { ptr::read_volatile((self.mapping.address() + offset) as *const u32) }
    }

    fn write_reg32(&self, offset: usize, value: u32) {
        assert!(offset + mem::size_of::<u32>() <= self.mapping.len());
        assert!(offset % mem::size_of::<u32>() == 0);

        unsafe { ptr::write_volatile((self.mapping.address() + offset) as *mut u32, value) }
    }

    fn read_reg64(&self, offset: usize) -> u64 {
        assert!(offset + mem::size_of::<u64>() <= self.mapping.len());
        assert!(offset % mem::size_of::<u64>() == 0);
        // Only possible on offset >= 0x80, see spec
        assert!(offset >= 0x80);

        unsafe { ptr::read_volatile((self.mapping.address() + offset) as *const u64) }
    }

    fn write_reg64(&self, offset: usize, value: u64) {
        assert!(offset + mem::size_of::<u64>() <= self.mapping.len());
        assert!(offset % mem::size_of::<u64>() == 0);
        // Only possible on offset >= 0x80, see spec
        assert!(offset >= 0x80);

        unsafe { ptr::write_volatile((self.mapping.address() + offset) as *mut u64, value) }
    }

    pub fn id(&self) -> u32 {
        self.read_reg32(registers::ID)
    }

    pub fn check_liveness(&self) -> bool {
        // It is a simple value inversion (~ C operator).
        let value = 0x1234;
        self.write_reg32(registers::LIVENESS, value);
        self.read_reg32(registers::LIVENESS) == !value
    }

    pub fn compute_factorial(&self, n: u32) -> u32 {
        self.write_reg32(registers::FACTORIAL, n);
        while self.read_reg32(registers::STATUS) & status::COMPUTING != 0 {}
        self.read_reg32(registers::FACTORIAL)
    }

    // TODO: Implement EDU device functionality
    // - Factorial computation
    // - Interrupt handling
    // - DMA operations
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    log::info!("edu-server starting...");

    // Find the EDU device on the PCI bus
    let pci_address = match find_edu_device() {
        Some(addr) => addr,
        None => {
            error!("EDU device not found, exiting");
            return 1;
        }
    };

    let edu = EduDevice::new(pci_address);

    log::info!("EDU device initialized successfully");

    log::info!("Edu device ID: 0x{:08x}", edu.id());
    log::info!(
        "Edu liveness check: {}",
        if edu.check_liveness() {
            "passed"
        } else {
            "failed"
        }
    );

    let n = 5;
    log::info!("Computing factorial of {} using EDU device...", n);
    let result = edu.compute_factorial(n);
    log::info!("Factorial of {} is {}", n, result);

    0
}

fn find_edu_device() -> Option<pci::PciAddress> {
    let options = pci::ListOptions::new();
    let options = options.with_device_id(EDU_VENDOR_ID, EDU_DEVICE_ID);
    let infos = pci::list(options).expect("Failed to list PCI devices");
    infos.first().map(|info| info.address)
}
