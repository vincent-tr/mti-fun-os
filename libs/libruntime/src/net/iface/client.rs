use crate::{drivers::pci::PciAddress, ipc, kobject::KObject};

use super::messages;

pub type NetServerCallError = ipc::CallError<messages::NetServerError>;

/// Low level net server client implementation.
#[derive(Debug)]
pub struct Client {
    ipc_client: ipc::Client<'static>,
}

impl Client {
    /// Creates a new net server client.
    pub fn new() -> Self {
        Self {
            ipc_client: ipc::Client::new(messages::PORT_NAME, messages::VERSION),
        }
    }

    /// Create a new network interface.
    pub fn create_interface(
        &self,
        name: &str,
        driver_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<(), NetServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();
        let (driver_port_name_mobj, driver_port_name_buffer) =
            ipc::Buffer::new_local(driver_port_name.as_bytes()).into_shared();

        let query = messages::CreateInterfaceQueryParameters {
            name: name_buffer,
            driver_port_name: driver_port_name_buffer,
            pci_address,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::CreateInterfaceQueryParameters::HANDLE_NAME_MOBJ] =
            name_mobj.into_handle();
        query_handles[messages::CreateInterfaceQueryParameters::HANDLE_DRIVER_PORT_NAME_MOBJ] =
            driver_port_name_mobj.into_handle();

        self.ipc_client.call::<
            messages::Type,
            messages::CreateInterfaceQueryParameters,
            messages::CreateInterfaceReply,
            messages::NetServerError,
        >(messages::Type::CreateInterface, query, query_handles)?;

        Ok(())
    }

    /// Destroy a network interface.
    pub fn destroy_interface(&self, name: &str) -> Result<(), NetServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::DestroyInterfaceQueryParameters { name: name_buffer };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::DestroyInterfaceQueryParameters::HANDLE_NAME_MOBJ] =
            name_mobj.into_handle();

        self.ipc_client.call::<
            messages::Type,
            messages::DestroyInterfaceQueryParameters,
            messages::DestroyInterfaceReply,
            messages::NetServerError,
        >(messages::Type::DestroyInterface, query, query_handles)?;

        Ok(())
    }
}
