use crate::{drivers::pci::PciAddress, ipc, kobject::KObject};

use super::messages;

pub type NetServerCallError = ipc::CallError<messages::NetError>;

/// Low level net server client implementation.
#[derive(Debug)]
pub struct Client {
    ipc_client: ipc::Client<'static>,
}

impl Client {
    /// Creates a new net server client.
    pub fn new(port_name: &'static str) -> Self {
        Self {
            ipc_client: ipc::Client::new(port_name, messages::VERSION),
        }
    }

    /// Create a new network device.
    pub fn create_device(
        &self,
        name: &str,
        driver_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<(), NetServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();
        let (driver_port_name_mobj, driver_port_name_buffer) =
            ipc::Buffer::new_local(driver_port_name.as_bytes()).into_shared();

        let query = messages::CreateDeviceQueryParameters {
            name: name_buffer,
            driver_port_name: driver_port_name_buffer,
            pci_address,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::CreateDeviceQueryParameters::HANDLE_NAME_MOBJ] =
            name_mobj.into_handle();
        query_handles[messages::CreateDeviceQueryParameters::HANDLE_DRIVER_PORT_NAME_MOBJ] =
            driver_port_name_mobj.into_handle();

        self.ipc_client.call::<
            messages::Type,
            messages::CreateDeviceQueryParameters,
            messages::CreateDeviceReply,
            messages::NetError,
        >(messages::Type::CreateDevice, query, query_handles)?;

        Ok(())
    }

    /// Destroy a network device.
    pub fn destroy_device(&self, name: &str) -> Result<(), NetServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::DestroyDeviceQueryParameters { name: name_buffer };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::DestroyDeviceQueryParameters::HANDLE_NAME_MOBJ] =
            name_mobj.into_handle();

        self.ipc_client.call::<
            messages::Type,
            messages::DestroyDeviceQueryParameters,
            messages::DestroyDeviceReply,
            messages::NetError,
        >(messages::Type::DestroyDevice, query, query_handles)?;

        Ok(())
    }
}
