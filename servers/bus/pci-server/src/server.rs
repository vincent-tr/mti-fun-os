use core::mem;

use alloc::{sync::Arc, vec::Vec};
use hashbrown::HashMap;
use libruntime::{
    drivers::pci::{
        iface::{self, PciServerError},
        types::{PciAddress, PciHeader},
    },
    ipc,
};
use log::{debug, error};

use crate::{device::Device, pci::scan_bus, state};

/// PCI Server
#[derive(Debug)]
pub struct Server {
    devices: HashMap<PciAddress, Arc<Device>>,
    handles: ipc::HandleTable<'static, Device>,
}

impl Server {
    /// Creates a new PCI server instance by opening the PCI configuration space access ports.
    pub fn new() -> Self {
        let mut server = Self {
            devices: HashMap::new(),
            handles: ipc::HandleTable::new(state::State::get().handle_generator()),
        };

        server.scan();

        server
    }

    fn scan(&mut self) {
        // TODO: PCI-to-PCI bridges, which may require scanning multiple buses.
        // For now we just scan bus 0.
        let addresses = scan_bus(0);

        for address in addresses {
            let device = Arc::new(Device::new(address));

            debug!(
                "Found PCI device: address {}, id {}, class {} ({})",
                device.address(),
                device.device_id(),
                device.class(),
                device.class().kind()
            );

            self.devices.insert(device.address(), device);
        }
    }
}

impl iface::PciServer for Server {
    type Error = iface::PciServerError;

    fn list(
        &self,
        _sender_id: u64,
        vendor_id: Option<u16>,
        device_id: Option<u16>,
        class: Option<u8>,
        subclass: Option<u8>,
    ) -> Result<Vec<iface::PciDeviceInfo>, PciServerError> {
        let mut list = Vec::new();

        for device in self.devices.values() {
            if let Some(vendor_id) = vendor_id
                && device.device_id().vendor != vendor_id
            {
                continue;
            }

            if let Some(device_id) = device_id
                && device.device_id().device != device_id
            {
                continue;
            }

            if let Some(class) = class
                && device.class().class != class
            {
                continue;
            }

            if let Some(subclass) = subclass
                && device.class().subclass != subclass
            {
                continue;
            }

            list.push(device.info());
        }

        Ok(list)
    }

    fn get_by_address(
        &self,
        _sender_id: u64,
        address: PciAddress,
    ) -> Result<iface::PciDeviceInfo, PciServerError> {
        if let Some(device) = self.devices.get(&address) {
            Ok(device.info())
        } else {
            Err(PciServerError::DeviceNotFound)
        }
    }

    fn open(
        &self,
        sender_id: u64,
        address: PciAddress,
    ) -> Result<libruntime::ipc::Handle, PciServerError> {
        let device = self
            .devices
            .get(&address)
            .ok_or(PciServerError::DeviceNotFound)?;

        if !device.try_open() {
            return Err(PciServerError::DeviceInUse);
        }

        let handle = self.handles.open(sender_id, device.clone());

        Ok(handle)
    }

    fn close(&self, sender_id: u64, handle: ipc::Handle) -> Result<(), PciServerError> {
        let device = self.handles.close(sender_id, handle).ok_or_else(|| {
            error!("Invalid device handle: {:?}", handle);
            PciServerError::InvalidArgument
        })?;

        device.closed();

        Ok(())
    }

    fn get_header(&self, sender_id: u64, handle: ipc::Handle) -> Result<PciHeader, Self::Error> {
        let device = self.handles.read(sender_id, handle).ok_or_else(|| {
            error!("Invalid device handle: {:?}", handle);
            PciServerError::InvalidArgument
        })?;

        Ok(device
            .header()
            .expect("Device header should be present for opened device"))
    }

    fn enable(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
        memory: bool,
        io: bool,
        bus_master: bool,
    ) -> Result<(), Self::Error> {
        let device = self.handles.read(sender_id, handle).ok_or_else(|| {
            error!("Invalid device handle: {:?}", handle);
            PciServerError::InvalidArgument
        })?;

        device.enable(memory, io, bus_master);

        Ok(())
    }
}
