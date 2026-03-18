use core::{mem, pin::Pin};

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use libruntime::{
    drivers::MmioRegion,
    kobject,
    net::{
        dev::iface::TxBufferDescriptor,
        types::{PhysAddr, PhysBufferPoolAccess},
    },
};

use crate::{descriptors, registers};

const TX_RING_SIZE: usize = 256;
const RX_RING_SIZE: usize = 256;

/// Tx ring for the e1000e driver.
#[derive(Debug)]
pub struct TxRing {
    /// Access to the physical buffer pool for this device, used to translate between physical addresses and buffer descriptors.
    buffer_pool: Arc<PhysBufferPoolAccess>,

    /// Access to the MMIO region for this device, used to read and write the Tx ring head and tail registers.
    mmio_region: Arc<MmioRegion<u32>>,

    /// The buffer descriptors for this Tx ring.
    descriptors: Pin<Box<TxDescriptors>>,

    /// The head index as computed by the driver.
    computed_head: usize,
}

#[derive(Debug)]
struct TxDescriptors([descriptors::TxDescriptor; TX_RING_SIZE]);

impl Default for TxDescriptors {
    fn default() -> Self {
        Self([descriptors::TxDescriptor::default(); TX_RING_SIZE])
    }
}

impl TxRing {
    /// Creates a new Tx ring with the given MMIO region and buffer pool access. This will also set up the Tx ring on the device, including writing the base address and length to the appropriate MMIO registers.
    pub fn new(mmio_region: Arc<MmioRegion<u32>>, buffer_pool: Arc<PhysBufferPoolAccess>) -> Self {
        const {
            assert!(
                mem::size_of::<TxDescriptors>() <= kobject::PAGE_SIZE,
                "Tx ring must fit within a single page"
            );

            assert!(
                TX_RING_SIZE % 128 == 0,
                "Tx ring size must be a multiple of 128"
            );

            assert!(
                mem::align_of::<TxDescriptors>() <= 16,
                "Tx ring must be aligned to at least 16 bytes"
            );
        }

        let ring = Self {
            buffer_pool,
            mmio_region,
            descriptors: Box::pin(TxDescriptors::default()),
            computed_head: 0,
        };

        let addr_info = kobject::Process::current()
            .map_info(&*ring.descriptors as *const _ as usize)
            .expect("Could not get memory info for descriptors buffer");
        let descriptors_phys_addr = PhysAddr::from(
            addr_info
                .mobj
                .expect("Could not get memory object from descriptors buffer info")
                .phys_addr(addr_info.offset)
                .expect("Could not get physical address for descriptors buffer"),
        );

        // Setup ring buffer
        let mut addr_low = registers::TxDescriptorBaseLow::default();
        addr_low.set_address(descriptors_phys_addr.as_u64() as u32);
        ring.mmio_region
            .write(registers::TxDescriptorBaseLow::OFFSET, addr_low.into());

        let mut addr_high = registers::TxDescriptorBaseHigh::default();
        addr_high.set_address((descriptors_phys_addr.as_u64() >> 32) as u32);
        ring.mmio_region
            .write(registers::TxDescriptorBaseHigh::OFFSET, addr_high.into());

        let mut length = registers::TxDescriptorLength::default();
        length.set_length(TX_RING_SIZE * mem::size_of::<descriptors::TxDescriptor>());
        ring.mmio_region
            .write(registers::TxDescriptorLength::OFFSET, length.into());

        let mut head = registers::TxDescriptorHead::default();
        head.set_index(0);
        ring.mmio_region
            .write(registers::TxDescriptorHead::OFFSET, head.into());

        let mut tail = registers::TxDescriptorTail::default();
        tail.set_index(0);
        ring.mmio_region
            .write(registers::TxDescriptorTail::OFFSET, tail.into());

        ring
    }

    /// Adds the given buffers to the Tx ring, returning the number of buffers that were added.
    pub fn add_buffers(&mut self) -> usize {
        todo!()
    }

    /// Processes completed transmissions, returning the addresses of the buffers that were transmitted.
    pub fn process_completions(&mut self, mmio_region: &MmioRegion<u32>) -> Vec<PhysAddr> {
        todo!()
    }
}
