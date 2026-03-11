use alloc::{boxed::Box, string::String};
use libruntime::{
    drivers::pci,
    net::{
        MacAddress,
        dev::{NetDevice, iface::NetDeviceError},
    },
};
use log::error;

/// Represents an E1000e network device.
pub struct E1000eDevice {
    name: String,
    pci_device: pci::PciDevice,
    link_status_change: Box<dyn Fn(bool) + Send + Sync + 'static>,
}

impl NetDevice for E1000eDevice {
    type Error = NetDeviceError;

    fn create(
        name: &str,
        pci_address: pci::PciAddress,
        link_status_change_callback: impl Fn(bool) + Send + Sync + 'static,
    ) -> Result<Box<Self>, Self::Error> {
        let pci_device = pci::PciDevice::open(pci_address).into_netdev_err()?;

        Ok(Box::new(Self {
            name: String::from(name),
            pci_device,
            link_status_change: Box::new(link_status_change_callback),
        }))
    }

    fn destroy(self) {
        todo!()
    }

    fn get_link_status(&self) -> Result<bool, Self::Error> {
        todo!()
    }

    fn get_mac_address(&self) -> Result<MacAddress, Self::Error> {
        todo!()
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
