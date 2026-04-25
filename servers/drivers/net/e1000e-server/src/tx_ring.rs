use core::{
    mem,
    ops::{Index, IndexMut},
    pin::Pin,
};

use alloc::{boxed::Box, fmt, sync::Arc, vec::Vec};
use libruntime::{
    kobject,
    net::{
        dev::{TX_FREE_BUFFER_COUNT, iface::TxBufferDescriptor},
        types::PhysAddr,
    },
};
use log::warn;

use crate::{descriptors, device::DeviceData, registers};

const TX_RING_SIZE: usize = 256;

/// Tx ring for the e1000e driver.
pub struct TxRing {
    dev_data: Arc<DeviceData>,

    /// The buffer descriptors for this Tx ring.
    descriptors: Pin<Box<TxDescriptors>>,

    /// The head index as computed by the driver.
    head: usize,

    /// The tail index as computed by the driver.
    tail: usize,

    /// Callback to notify the server of completed transmissions, passing the buffer indexes of the completed buffers.
    buffer_free: Box<dyn Fn(&[usize]) + Send + Sync + 'static>,
}

impl fmt::Debug for TxRing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxRing")
            .field("descriptors", &self.descriptors)
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}

// Force it to be page-aligned, to ensure that the whole buffer physical layout is contigous
#[derive(Debug)]
#[repr(align(4096))]
struct TxDescriptors([descriptors::TxDescriptor; TX_RING_SIZE]);

impl Default for TxDescriptors {
    fn default() -> Self {
        Self([descriptors::TxDescriptor::default(); TX_RING_SIZE])
    }
}

impl Index<usize> for TxDescriptors {
    type Output = descriptors::TxDescriptor;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for TxDescriptors {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl TxRing {
    /// Creates a new Tx ring with the given MMIO region and buffer pool access. This will also set up the Tx ring on the device, including writing the base address and length to the appropriate MMIO registers.
    pub fn new(
        dev_data: Arc<DeviceData>,
        buffer_free: impl Fn(&[usize]) + Send + Sync + 'static,
    ) -> Self {
        const {
            assert!(
                mem::size_of::<TxDescriptors>() <= kobject::PAGE_SIZE,
                "Tx ring must fit within a single page"
            );

            assert!(
                mem::align_of::<TxDescriptors>() == kobject::PAGE_SIZE,
                "Rx ring must be aligned to page size"
            );

            assert!(
                TX_RING_SIZE % 128 == 0,
                "Tx ring size must be a multiple of 128"
            );
        }

        Self {
            dev_data,
            descriptors: Box::pin(TxDescriptors::default()),
            head: 0,
            tail: 0,
            buffer_free: Box::new(buffer_free),
        }
    }

    /// Initializes the Tx ring with the hardware.
    pub fn init(&mut self) {
        let addr_info = kobject::Process::current()
            .map_info(&*self.descriptors as *const _ as usize)
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
        self.dev_data
            .mmio_write(registers::TxDescriptorBaseLow::OFFSET, addr_low);

        let mut addr_high = registers::TxDescriptorBaseHigh::default();
        addr_high.set_address((descriptors_phys_addr.as_u64() >> 32) as u32);
        self.dev_data
            .mmio_write(registers::TxDescriptorBaseHigh::OFFSET, addr_high);

        let mut length = registers::TxDescriptorLength::default();
        length.set_length(TX_RING_SIZE * mem::size_of::<descriptors::TxDescriptor>());
        self.dev_data
            .mmio_write(registers::TxDescriptorLength::OFFSET, length);

        let mut head = registers::TxDescriptorHead::default();
        head.set_index(self.head);
        self.dev_data
            .mmio_write(registers::TxDescriptorHead::OFFSET, head);

        let mut tail = registers::TxDescriptorTail::default();
        tail.set_index(self.tail);
        self.dev_data
            .mmio_write(registers::TxDescriptorTail::OFFSET, tail);

        let mut control = registers::TxControl::default();
        control.enable(true);
        control.set_pad_short_packets(true);
        self.dev_data
            .mmio_write(registers::TxControl::OFFSET, control);
    }

    /// Adds the given buffers to the Tx ring, returning the number of buffers that were added.
    pub fn add_buffers(&mut self, buffers: &[TxBufferDescriptor]) -> usize {
        let freed_indexes = self.process_completions();

        let mut added = 0;

        for buffer in buffers {
            let next_index = (self.tail + 1) % TX_RING_SIZE;
            if next_index == self.head {
                // Ring is full, stop adding buffers
                break;
            }

            self.set_buffer(self.tail, buffer);

            self.tail = next_index;
            added += 1;
        }

        if added > 0 {
            // Update tail register to notify device of new buffers
            let mut tail = registers::TxDescriptorTail::default();
            tail.set_index(self.tail);
            self.dev_data
                .mmio_write(registers::TxDescriptorTail::OFFSET, tail);
        }

        // Notifiy server of completed buffers
        self.notify_completions(freed_indexes);

        added
    }

    fn set_buffer(&mut self, index: usize, buffer: &TxBufferDescriptor) {
        let desc = &mut self.descriptors[index];
        *desc = descriptors::TxDescriptor::default();

        let phys_addr = self.dev_data.buffer_address_of(buffer.buffer_index());

        desc.set_address(phys_addr + buffer.offset());
        desc.set_length(buffer.length());

        let mut cmd = descriptors::TxDescriptorCommand::default();
        cmd.set_end_of_packet(buffer.end_of_packet());
        desc.set_command(cmd);
    }

    /// Handles a Tx queue empty interrupt by processing completed transmissions.
    pub fn handle_queue_empty_interrupt(&mut self) {
        let freed_indexes = self.process_completions();

        self.notify_completions(freed_indexes);
    }

    fn process_completions(&mut self) -> Vec<usize> {
        let mut completed_buffers = Vec::new();

        loop {
            let desc = &self.descriptors[self.head];
            let status = desc.status();

            if !status.descriptor_done() {
                // No more completed buffers
                break;
            }

            if status.excess_collisions() {
                warn!(
                    "Got TX excess collisions at descriptor {} for NIC {}",
                    self.head,
                    self.dev_data.name()
                );
            }

            if status.late_collision() {
                warn!(
                    "Got TX late collision at descriptor {} for NIC {}",
                    self.head,
                    self.dev_data.name()
                );
            }

            if status.transmit_underrun() {
                warn!(
                    "Got TX transmit underrun at descriptor {} for NIC {}",
                    self.head,
                    self.dev_data.name()
                );
            }

            // Buffer at head index has been transmitted, add it to the list of completed buffers
            let phys_addr = desc.address();
            let (buffer_index, _) = self.dev_data.buffer_index_of(phys_addr);

            completed_buffers.push(buffer_index);

            self.descriptors[self.head] = descriptors::TxDescriptor::default();

            self.head = (self.head + 1) % TX_RING_SIZE;
        }

        completed_buffers
    }

    fn notify_completions(&self, freed_indexes: Vec<usize>) {
        for chunk in freed_indexes.chunks(TX_FREE_BUFFER_COUNT) {
            (self.buffer_free)(chunk);
        }
    }
}
