use core::pin::Pin;

use alloc::boxed::Box;

use super::messages;
use crate::{
    drivers::pci::PciAddress,
    ipc,
    kobject::{self, KObject},
    net::types::MacAddress,
};

pub type NetDeviceServerCallError = ipc::CallError<messages::NetDeviceError>;

/// Low level net device client implementation.
#[derive(Debug)]
pub struct Client<'a> {
    port_name: Pin<Box<str>>,
    ipc_client: ipc::Client<'a>,
}

impl Client<'_> {
    /// Creates a new net device client.
    pub fn new(port: &str) -> Self {
        let port_name: Box<str> = port.into();
        let port_name = Box::into_pin(port_name);

        // Safety: The port_name is owned by this Client and will not be modified or dropped while ipc_client is using it.
        let port_name_ref = unsafe { &*(Pin::get_ref(port_name.as_ref()) as *const str) };

        Self {
            port_name: port_name,
            ipc_client: ipc::Client::new(port_name_ref, messages::VERSION),
        }
    }

    /// Create a new network device.
    pub fn create(&self, pci_address: PciAddress) -> Result<ipc::Handle, NetDeviceServerCallError> {
        let query = messages::CreateQueryParameters { pci_address };
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::CreateQueryParameters,
            messages::CreateReply,
            messages::NetDeviceError,
        >(messages::Type::Create, query, query_handles)?;

        Ok(reply.handle)
    }

    /// Destroy a network device.
    pub fn destroy(&self, handle: ipc::Handle) -> Result<(), NetDeviceServerCallError> {
        let query = messages::DestroyQueryParameters { handle };
        let query_handles = ipc::KHandles::new();

        let (_reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::DestroyQueryParameters,
            messages::DestroyReply,
            messages::NetDeviceError,
        >(messages::Type::Destroy, query, query_handles)?;

        Ok(())
    }

    /// Get the link status of a network device.
    pub fn get_link_status(&self, handle: ipc::Handle) -> Result<bool, NetDeviceServerCallError> {
        let query = messages::GetLinkStatusQueryParameters { handle };
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::GetLinkStatusQueryParameters,
            messages::GetLinkStatusReply,
            messages::NetDeviceError,
        >(messages::Type::GetLinkStatus, query, query_handles)?;

        Ok(reply.link_up)
    }

    /// Register for link status change notifications.
    pub fn register_link_status_change(
        &self,
        handle: ipc::Handle,
        port: kobject::PortSender,
        correlation: u64,
    ) -> Result<ipc::Handle, NetDeviceServerCallError> {
        let query = messages::RegisterLinkStatusChangeQueryParameters {
            handle,
            correlation,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::RegisterLinkStatusChangeQueryParameters::HANDLE_PORT] =
            port.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::RegisterLinkStatusChangeQueryParameters,
            messages::RegisterLinkStatusChangeReply,
            messages::NetDeviceError,
        >(
            messages::Type::RegisterLinkStatusChange,
            query,
            query_handles,
        )?;

        Ok(reply.registration_handle)
    }

    /// Unregister from link status change notifications.
    pub fn unregister_link_status_change(
        &self,
        registration_handle: ipc::Handle,
    ) -> Result<(), NetDeviceServerCallError> {
        let query = messages::UnregisterLinkStatusChangeQueryParameters {
            registration_handle,
        };
        let query_handles = ipc::KHandles::new();

        let (_reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::UnregisterLinkStatusChangeQueryParameters,
            messages::UnregisterLinkStatusChangeReply,
            messages::NetDeviceError,
        >(
            messages::Type::UnregisterLinkStatusChange,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// Get the MAC address of a network device.
    pub fn get_mac_address(
        &self,
        handle: ipc::Handle,
    ) -> Result<MacAddress, NetDeviceServerCallError> {
        let query = messages::GetMacAddressQueryParameters { handle };
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::GetMacAddressQueryParameters,
            messages::GetMacAddressReply,
            messages::NetDeviceError,
        >(messages::Type::GetMacAddress, query, query_handles)?;

        Ok(reply.mac_address)
    }
}
