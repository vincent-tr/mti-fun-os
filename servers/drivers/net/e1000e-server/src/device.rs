use core::panic;
use spin::Mutex;

use crate::{
    eeprom::EepromAccess,
    link_status::LinkStatus,
    registers,
    tx_ring::{self, TxRing},
};
use alloc::{boxed::Box, string::String, sync::Arc};
use libruntime::{
    drivers::{MmioRegion, pci},
    kobject,
    net::{
        dev::{
            NetDevice,
            iface::{NetDeviceError, RxBufferDescriptor, TxBufferDescriptor},
        },
        types::{BufferPool, MacAddress, PhysAddr, PhysBufferPoolAccess},
    },
};
use log::{debug, error};

/// Represents an E1000e network device.
#[derive(Debug)]
pub struct E1000eDevice {
    dev_data: Arc<DeviceData>,
    pci_device: pci::PciDevice,
    link_status: Mutex<LinkStatus>,
    tx_ring: Mutex<TxRing>,
}

impl NetDevice for E1000eDevice {
    type Error = NetDeviceError;

    fn create(
        name: &str,
        pci_address: pci::PciAddress,
        buffer_pool: BufferPool,
        link_status_change_callback: impl Fn(bool) + Send + Sync + 'static,
        tx_free_callback: impl Fn(&[usize]) + Send + Sync + 'static,
        rx_arrived_callback: impl Fn(&[RxBufferDescriptor]) + Send + Sync + 'static,
    ) -> Result<Box<Self>, Self::Error> {
        let pci_device = pci::PciDevice::open(pci_address).into_netdev_err()?;
        let header = pci_device.header().into_netdev_err()?;

        let Some(pci::Bar::Memory(bar0)) = header.bars[0] else {
            error!(
                "Unexpected BAR type for E1000e device: {:?}",
                header.bars[0]
            );
            return Err(NetDeviceError::DeviceError);
        };

        let dev_data = DeviceData::new(
            name,
            MmioRegion::<u32>::from_bar(&bar0).into_netdev_err()?,
            &buffer_pool,
        );

        let link_status = Mutex::new(LinkStatus::new(link_status_change_callback));
        let tx_ring = Mutex::new(tx_ring::TxRing::new(dev_data.clone(), tx_free_callback));

        let device = Self {
            dev_data,
            pci_device,
            link_status,
            tx_ring,
        };

        device.init()?;

        Ok(Box::new(device))
    }

    fn destroy(self) {}

    fn get_link_status(&self) -> Result<bool, Self::Error> {
        let link_status = self.link_status.lock();

        Ok(link_status.is_up())
    }

    fn get_mac_address(&self) -> Result<MacAddress, Self::Error> {
        // MAC is stored in EEPROM words 0x00, 0x01, 0x02 (3 words = 6 bytes)
        let access = EepromAccess::acquire(&self.dev_data)?;
        let word0 = access.read(0x00)?;
        let word1 = access.read(0x01)?;
        let word2 = access.read(0x02)?;

        // Each word is 16 bits (2 bytes), stored in little-endian
        let [b0, b1] = word0.to_le_bytes();
        let [b2, b3] = word1.to_le_bytes();
        let [b4, b5] = word2.to_le_bytes();

        let mac = MacAddress::from([b0, b1, b2, b3, b4, b5]);

        Ok(mac)
    }

    fn tx(&self, descriptors: &[TxBufferDescriptor]) -> Result<usize, Self::Error> {
        let mut ring = self.tx_ring.lock();

        let added_count = ring.add_buffers(descriptors);

        Ok(added_count)
    }

    fn add_rx_buffers(&self, buffer_indexes: &[usize]) -> Result<usize, Self::Error> {
        todo!()
    }
}

impl E1000eDevice {
    fn init(&self) -> Result<(), NetDeviceError> {
        self.pci_device.enable(true, true, true).into_netdev_err()?;

        // Reset the device
        let mut control: registers::Control = self.dev_data.mmio_read(registers::Control::OFFSET);
        control.set_reset(true);
        self.dev_data
            .mmio_write(registers::Control::OFFSET, control);

        loop {
            let control: registers::Control = self.dev_data.mmio_read(registers::Control::OFFSET);
            if !control.reset() {
                break;
            }

            core::hint::spin_loop();
        }

        // Setup
        let mut control: registers::Control = self.dev_data.mmio_read(registers::Control::OFFSET);
        control.set_auto_speed_detection(true);
        control.set_link_up(true);
        self.dev_data
            .mmio_write(registers::Control::OFFSET, control);

        // Setup MAC Address
        let address = self.get_mac_address()?;
        debug!("Setting MAC address to {}", address);
        let mut ral0 = registers::RxAddressLow::default();
        ral0.set_address(u32::from_le_bytes([
            address[0], address[1], address[2], address[3],
        ]));
        self.dev_data
            .mmio_write(registers::RxAddressLow::OFFSET0, ral0);

        let mut rah0 = registers::RxAddressHigh::default();
        rah0.set_address(u32::from_le_bytes([address[4], address[5], 0, 0]));
        rah0.set_valid(true);
        rah0.set_address_select(registers::AddressSelect::Destination);
        self.dev_data
            .mmio_write(registers::RxAddressHigh::OFFSET0, rah0);

        // Setup Tx ring
        self.tx_ring.lock().init();

        // TODO: RING SETUP, INTERRUPT SETUP

        panic!("E1000e device creation not implemented yet");
    }
}

/// Common data for the device, shared between different parts of the driver implementation.
#[derive(Debug)]
pub struct DeviceData {
    name: String,
    mmio_region: MmioRegion<u32>,
    buffer_pool: PhysBufferPoolAccess,
}

impl DeviceData {
    /// Create a new DeviceData instance.
    pub fn new(name: &str, mmio_region: MmioRegion<u32>, buffer_pool: &BufferPool) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            mmio_region,
            buffer_pool: PhysBufferPoolAccess::new(buffer_pool),
        })
    }

    /// Get the name of the device, used for logging.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Read a register from the MMIO region.
    pub fn mmio_read<Register>(&self, offset: usize) -> Register
    where
        Register: From<u32>,
    {
        Register::from(self.mmio_region.read(offset))
    }

    /// Write a register to the MMIO region.
    pub fn mmio_write<Register>(&self, offset: usize, value: Register)
    where
        Register: Into<u32>,
    {
        self.mmio_region.write(offset, value.into());
    }

    /// Get the physical address of a buffer in the buffer pool by its index.
    pub fn buffer_address_of(&self, buffer_index: usize) -> PhysAddr {
        self.buffer_pool
            .address_of(buffer_index)
            .expect("Bad buffer index")
    }

    /// Get the buffer index and offset for a given physical address.
    pub fn buffer_index_of(&self, addr: PhysAddr) -> (usize, usize) {
        self.buffer_pool
            .index_of(addr)
            .expect("Address not part of any buffer")
    }
}

trait ResultExt<T> {
    fn into_netdev_err(self) -> Result<T, NetDeviceError>;
}

impl<T> ResultExt<T> for Result<T, pci::PciServerCallError> {
    fn into_netdev_err(self) -> Result<T, NetDeviceError> {
        self.map_err(|e| {
            error!("PCI server call failed: {:?}", e);

            match e {
                pci::PciServerCallError::KernelError(_) => NetDeviceError::RuntimeError,
                pci::PciServerCallError::ReplyError(pci::PciServerError::InvalidArgument) => {
                    NetDeviceError::InvalidArgument
                }
                pci::PciServerCallError::ReplyError(pci::PciServerError::RuntimeError) => {
                    NetDeviceError::RuntimeError
                }
                pci::PciServerCallError::ReplyError(_) => NetDeviceError::DeviceError,
            }
        })
    }
}

impl<T> ResultExt<T> for Result<T, kobject::Error> {
    fn into_netdev_err(self) -> Result<T, NetDeviceError> {
        self.map_err(|e| {
            error!("Kernel operation failed: {:?}", e);
            NetDeviceError::RuntimeError
        })
    }
}
