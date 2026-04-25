use core::{
    mem,
    ops::{Index, IndexMut},
    pin::Pin,
};

use alloc::{boxed::Box, fmt, sync::Arc, vec::Vec};
use libruntime::{
    kobject,
    net::{
        dev::{RX_ARRIVED_DESCRIPTOR_COUNT, iface::RxBufferDescriptor},
        types::PhysAddr,
    },
};
use log::warn;

use crate::{descriptors, device::DeviceData, registers};

const RX_RING_SIZE: usize = 256;

/// Rx ring for the e1000e driver.
pub struct RxRing {
    dev_data: Arc<DeviceData>,

    /// The buffer descriptors for this Rx ring.
    descriptors: Pin<Box<RxDescriptors>>,

    /// The head index as computed by the driver.
    head: usize,

    /// The tail index as computed by the driver.
    tail: usize,

    /// Callback to notify the server of completed transmissions, passing the buffer indexes of the completed buffers.
    on_receive: Box<dyn Fn(&[RxBufferDescriptor]) + Send + Sync + 'static>,
}

impl fmt::Debug for RxRing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RxRing")
            .field("descriptors", &self.descriptors)
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}

// Force it to be page-aligned, to ensure that the whole buffer physical layout is contigous
#[derive(Debug)]
#[repr(align(4096))]
struct RxDescriptors([descriptors::RxDescriptor; RX_RING_SIZE]);

impl Default for RxDescriptors {
    fn default() -> Self {
        Self([descriptors::RxDescriptor::default(); RX_RING_SIZE])
    }
}

impl Index<usize> for RxDescriptors {
    type Output = descriptors::RxDescriptor;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for RxDescriptors {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl RxRing {
    /// Creates a new Rx ring with the given MMIO region and buffer pool access. This will also set up the Rx ring on the device, including writing the base address and length to the appropriate MMIO registers.
    pub fn new(
        dev_data: Arc<DeviceData>,
        on_receive: impl Fn(&[RxBufferDescriptor]) + Send + Sync + 'static,
    ) -> Self {
        const {
            assert!(
                mem::size_of::<RxDescriptors>() <= kobject::PAGE_SIZE,
                "Rx ring must fit within a single page"
            );

            assert!(
                mem::align_of::<RxDescriptors>() == kobject::PAGE_SIZE,
                "Rx ring must be aligned to page size"
            );

            assert!(
                RX_RING_SIZE % 128 == 0,
                "Rx ring size must be a multiple of 128"
            );
        }

        Self {
            dev_data,
            descriptors: Box::pin(RxDescriptors::default()),
            head: 0,
            tail: 0,
            on_receive: Box::new(on_receive),
        }
    }

    /// Initializes the Rx ring with the hardware.
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
        let mut addr_low = registers::RxDescriptorBaseLow::default();
        addr_low.set_address(descriptors_phys_addr.as_u64() as u32);
        self.dev_data
            .mmio_write(registers::RxDescriptorBaseLow::OFFSET, addr_low);

        let mut addr_high = registers::RxDescriptorBaseHigh::default();
        addr_high.set_address((descriptors_phys_addr.as_u64() >> 32) as u32);
        self.dev_data
            .mmio_write(registers::RxDescriptorBaseHigh::OFFSET, addr_high);

        let mut length = registers::RxDescriptorLength::default();
        length.set_length(RX_RING_SIZE * mem::size_of::<descriptors::RxDescriptor>());
        self.dev_data
            .mmio_write(registers::RxDescriptorLength::OFFSET, length);

        let mut head = registers::RxDescriptorHead::default();
        head.set_index(self.head);
        self.dev_data
            .mmio_write(registers::RxDescriptorHead::OFFSET, head);

        let mut tail = registers::RxDescriptorTail::default();
        tail.set_index(self.tail);
        self.dev_data
            .mmio_write(registers::RxDescriptorTail::OFFSET, tail);

        let mut control = registers::RxControl::default();
        control.enable(true);
        control.enable_long_packet_reception(true);
        control.set_broadcast_accepted(true);
        control.set_buffer_size(self.dev_data.buffer_size());
        control.set_strip_ethernet_crc(true); // Strip Ethernet FCS
        self.dev_data
            .mmio_write(registers::RxControl::OFFSET, control);
    }

    /// Adds the given buffers to the Rx ring, returning the number of buffers that were added.
    pub fn add_buffers(&mut self, buffer_indexes: &[usize]) -> usize {
        let mut added = 0;
        for &buffer_index in buffer_indexes {
            let next_index = (self.tail + 1) % RX_RING_SIZE;
            if next_index == self.head {
                // Ring is full
                break;
            }

            let descriptor = &mut self.descriptors[self.tail];
            *descriptor = descriptors::RxDescriptor::default();

            let addr = self.dev_data.buffer_address_of(buffer_index);
            descriptor.set_address(addr);

            added += 1;
            self.tail = next_index;
        }

        if added > 0 {
            let mut tail = registers::RxDescriptorTail::default();
            tail.set_index(self.tail);
            self.dev_data
                .mmio_write(registers::RxDescriptorTail::OFFSET, tail);
        }

        added
    }

    /// Handles a Rx ready interrupt
    pub fn handle_ready_interrupt(&mut self) {
        let mut receive_list = Vec::new();

        loop {
            let descriptor = &self.descriptors[self.head];
            if !descriptor.status().descriptor_done() {
                // No more completed descriptors
                break;
            }

            self.process_packet(descriptor, &mut receive_list);

            self.descriptors[self.head] = descriptors::RxDescriptor::default();

            self.head = (self.head + 1) % RX_RING_SIZE;
        }

        for chunk in receive_list.chunks(RX_ARRIVED_DESCRIPTOR_COUNT) {
            (self.on_receive)(chunk);
        }
    }

    fn process_packet(
        &self,
        descriptor: &descriptors::RxDescriptor,
        list: &mut Vec<RxBufferDescriptor>,
    ) {
        let eop = descriptor.status().end_of_packet();
        let error = descriptor.errors().any();
        let (buffer_index, _) = self.dev_data.buffer_index_of(descriptor.address());
        let length = descriptor.length();

        list.push(RxBufferDescriptor::new(buffer_index, length, eop, error));

        if error {
            warn!("Received packet with error {:?}", descriptor.errors());
        }
    }
}
