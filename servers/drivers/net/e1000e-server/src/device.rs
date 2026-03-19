use spin::Mutex;

use crate::{
    RUNNER, eeprom::EepromAccess, link_status::LinkStatus, registers, rx_ring::RxRing,
    tx_ring::TxRing,
};
use alloc::{boxed::Box, string::String, sync::Arc};
use libruntime::{
    drivers::{self, MmioRegion, pci},
    kobject,
    net::{
        dev::{
            NetDevice,
            iface::{NetDeviceError, RxBufferDescriptor, TxBufferDescriptor},
        },
        types::{BufferPool, MacAddress, PhysAddr, PhysBufferPoolAccess},
    },
};
use log::{debug, error, warn};

/// Represents an E1000e network device.
#[derive(Debug)]
pub struct E1000eDevice {
    dev_data: Arc<DeviceData>,
    pci_device: pci::PciDevice,
    irq: drivers::RunnableIrq,
    link_status: Mutex<LinkStatus>,
    tx_ring: Mutex<TxRing>,
    rx_ring: Mutex<RxRing>,
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

        let link_status = Mutex::new(LinkStatus::new(
            dev_data.clone(),
            link_status_change_callback,
        ));
        let tx_ring = Mutex::new(TxRing::new(dev_data.clone(), tx_free_callback));
        let rx_ring = Mutex::new(RxRing::new(dev_data.clone(), rx_arrived_callback));

        let irq = drivers::RunnableIrq::create(&RUNNER).into_netdev_err()?;

        let device = Box::new(Self {
            dev_data,
            pci_device,
            link_status,
            tx_ring,
            rx_ring,
            irq,
        });

        device.irq.set_callback({
            let callback = InterruptCallback::new(&device);
            move |event: kobject::IrqEvent| callback.call(event)
        });

        device.init()?;

        Ok(device)
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
        let mut ring = self.rx_ring.lock();

        let added_count = ring.add_buffers(buffer_indexes);

        Ok(added_count)
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
        self.rx_ring.lock().init();

        // Setup IRQ
        let irq_info = self.irq.info().into_netdev_err()?;
        self.pci_device.enable_msi(&irq_info).into_netdev_err()?;

        let mut imc = registers::InterruptMask::default();
        imc.set_link_status_change(true);
        imc.set_receive_timer_interrupt(true);
        imc.set_receive_fifo_overrun(true);
        imc.set_transmit_queue_empty(true);
        self.dev_data
            .mmio_write(registers::InterruptMask::OFFSET, imc);

        Ok(())
    }

    fn handle_interrupt(&self, _event: kobject::IrqEvent) {
        let cause: registers::InterruptCause =
            self.dev_data.mmio_read(registers::InterruptCause::OFFSET);

        // debug!(
        //     "Interrupt received on {}, cause: {:?}",
        //     self.dev_data.name(),
        //     cause
        // );

        if cause.rx_overrun() {
            warn!(
                "Receive FIFO overrun on NIC {}, some packets may have been dropped",
                self.dev_data.name()
            );
        }

        if cause.link_status_change() {
            self.link_status.lock().handle_interrupt();
        }

        if cause.rx_timer() {
            self.rx_ring.lock().handle_ready_interrupt();
        }

        if cause.tx_queue_empty() {
            self.tx_ring.lock().handle_queue_empty_interrupt();
        }
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

    /// Get the size of buffers in the buffer pool.
    pub fn buffer_size(&self) -> usize {
        self.buffer_pool.buffer_size()
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

#[derive(Debug, Clone, Copy)]
struct InterruptCallback {
    target: *const E1000eDevice,
}

unsafe impl Send for InterruptCallback {}
unsafe impl Sync for InterruptCallback {}

impl InterruptCallback {
    pub fn new(target: &E1000eDevice) -> Self {
        Self {
            target: target as *const E1000eDevice,
        }
    }

    pub fn call(&self, event: kobject::IrqEvent) {
        unsafe { (*self.target).handle_interrupt(event) }
    }
}
