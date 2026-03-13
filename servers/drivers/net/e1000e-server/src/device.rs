use core::{
    fmt, panic,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::registers;
use alloc::{boxed::Box, string::String};
use libruntime::{
    drivers::{MmioRegion, pci},
    kobject,
    net::{
        MacAddress,
        dev::{NetDevice, iface::NetDeviceError},
    },
};
use log::{error, info};

/// Represents an E1000e network device.
pub struct E1000eDevice {
    name: String,
    pci_device: pci::PciDevice,
    mmio_region: MmioRegion<u32>,
    link_status: LinkStatus,
}

impl fmt::Debug for E1000eDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("E1000eDevice")
            .field("name", &self.name)
            .field("pci_device", &self.pci_device)
            .field("link_status", &self.link_status)
            .finish()
    }
}

impl NetDevice for E1000eDevice {
    type Error = NetDeviceError;

    fn create(
        name: &str,
        pci_address: pci::PciAddress,
        link_status_change_callback: impl Fn(bool) + Send + Sync + 'static,
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

        let region = MmioRegion::<u32>::from_bar(&bar0).into_netdev_err()?;

        let control = registers::Control::from(region.read(registers::Control::OFFSET));
        let status = registers::Status::from(region.read(registers::Status::OFFSET));
        let rx_control = registers::RxControl::from(region.read(registers::RxControl::OFFSET));
        let tx_control = registers::TxControl::from(region.read(registers::TxControl::OFFSET));

        log::debug!("Control: {:?}", control);
        log::debug!("Status: {:?}", status);
        log::debug!("RxControl: {:?}", rx_control);
        log::debug!("TxControl: {:?}", tx_control);

        let device = Self {
            name: String::from(name),
            pci_device,
            mmio_region: region,
            link_status: LinkStatus::new(link_status_change_callback),
        };

        // Read and log the MAC address from EEPROM
        match device.get_mac_address() {
            Ok(mac) => log::info!("MAC address: {}", mac),
            Err(e) => log::error!("Failed to read MAC address: {:?}", e),
        }

        panic!("E1000e device creation not implemented yet");

        Ok(Box::new(device))
    }

    fn destroy(self) {}

    fn get_link_status(&self) -> Result<bool, Self::Error> {
        Ok(self.link_status.is_up())
    }

    fn get_mac_address(&self) -> Result<MacAddress, Self::Error> {
        // MAC is stored in EEPROM words 0x00, 0x01, 0x02 (3 words = 6 bytes)
        let access = EepromAccess::acquire(&self.mmio_region)?;
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
}

struct LinkStatus {
    is_up: AtomicBool,
    change: Box<dyn Fn(bool) + Send + Sync + 'static>,
}

impl fmt::Debug for LinkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinkStatus")
            .field("is_up", &self.is_up())
            .finish()
    }
}

impl LinkStatus {
    pub fn new(change_callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        Self {
            is_up: AtomicBool::new(false),
            change: Box::new(change_callback),
        }
    }

    pub fn update(&mut self, new_status: bool) {
        let old_status = self.is_up.swap(new_status, Ordering::SeqCst);
        if old_status != new_status {
            (self.change)(new_status);
        }
    }

    pub fn is_up(&self) -> bool {
        self.is_up.load(Ordering::SeqCst)
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

/// Access to the EEPROM
#[derive(Debug)]
struct EepromAccess<'a> {
    mmio_region: &'a MmioRegion<u32>,
}

impl Drop for EepromAccess<'_> {
    fn drop(&mut self) {
        let mut control = registers::EepromControlData::from(
            self.mmio_region.read(registers::EepromControlData::OFFSET),
        );
        control.set_access_request(false);
        self.mmio_region
            .write(registers::EepromControlData::OFFSET, control.into());
    }
}

impl<'a> EepromAccess<'a> {
    pub fn acquire(mmio_region: &'a MmioRegion<u32>) -> Result<Self, NetDeviceError> {
        let mut control = registers::EepromControlData::from(
            mmio_region.read(registers::EepromControlData::OFFSET),
        );

        if !control.present() {
            error!("EEPROM not present");
            return Err(NetDeviceError::DeviceError);
        }

        control.set_access_request(true);
        mmio_region.write(registers::EepromControlData::OFFSET, control.into());

        const MAX_ATTEMPTS: usize = 1000;

        let mut granted = false;
        let granted = for _ in 0..MAX_ATTEMPTS {
            let control = registers::EepromControlData::from(
                mmio_region.read(registers::EepromControlData::OFFSET),
            );
            if control.access_grant() {
                granted = true;
                break;
            }

            core::hint::spin_loop();
        };

        info!("Could not acquire EEPROM lock, consider it is not implemented");

        Ok(Self { mmio_region })
    }

    pub fn read(&self, address: u16) -> Result<u16, NetDeviceError> {
        let mut eerd = registers::EepromRead::default();
        eerd.set_address(address);
        eerd.set_start(true);
        self.mmio_region
            .write(registers::EepromRead::OFFSET, eerd.into());

        const MAX_ATTEMPTS: usize = 1000;

        for _ in 0..MAX_ATTEMPTS {
            let eerd =
                registers::EepromRead::from(self.mmio_region.read(registers::EepromRead::OFFSET));

            if eerd.done() {
                return Ok(eerd.data());
            }

            core::hint::spin_loop();
        }

        error!("EEPROM read timeout for address {:#x}", address);
        Err(NetDeviceError::DeviceError)
    }
}
