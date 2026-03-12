use core::{fmt, panic};

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
use log::error;

/// Represents an E1000e network device.
#[derive(Debug)]
pub struct E1000eDevice {
    name: String,
    pci_device: pci::PciDevice,
    link_status: LinkStatus,
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

        log::debug!("Opening BAR0: {:?}", bar0);
        let region = MmioRegion::<u32>::from_bar(&bar0).into_netdev_err()?;

        let control = registers::Control::from(region.read(registers::Control::OFFSET));
        let status = registers::Status::from(region.read(registers::Status::OFFSET));
        let rx_control = registers::RxControl::from(region.read(registers::RxControl::OFFSET));
        let tx_control = registers::TxControl::from(region.read(registers::TxControl::OFFSET));

        log::debug!("Control: {:?}", control);
        log::debug!("Status: {:?}", status);
        log::debug!("RxControl: {:?}", rx_control);
        log::debug!("TxControl: {:?}", tx_control);

        panic!("E1000e device creation not implemented yet");

        Ok(Box::new(Self {
            name: String::from(name),
            pci_device,
            link_status: LinkStatus::new(link_status_change_callback),
        }))
    }

    fn destroy(self) {}

    fn get_link_status(&self) -> Result<bool, Self::Error> {
        Ok(self.link_status.is_up())
    }

    fn get_mac_address(&self) -> Result<MacAddress, Self::Error> {
        todo!()
    }
}

struct LinkStatus {
    is_up: bool,
    change: Box<dyn Fn(bool) + Send + Sync + 'static>,
}

impl fmt::Debug for LinkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinkStatus")
            .field("is_up", &self.is_up)
            .finish()
    }
}

impl LinkStatus {
    pub fn new(change_callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        Self {
            is_up: false,
            change: Box::new(change_callback),
        }
    }

    pub fn update(&mut self, new_status: bool) {
        if self.is_up != new_status {
            self.is_up = new_status;
            (self.change)(new_status);
        }
    }

    pub fn is_up(&self) -> bool {
        self.is_up
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
