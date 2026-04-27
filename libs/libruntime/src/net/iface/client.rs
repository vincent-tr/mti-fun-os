use alloc::vec::Vec;

use crate::{
    drivers::pci::PciAddress,
    ipc,
    kobject::KObject,
    net::{
        iface::{InterfaceConfig, InterfaceInfo, Route, RoutesBlock},
        types::{IpAddress, IpPrefix},
    },
};

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

    /// Set inteface configuration
    pub fn set_interface_config(
        &self,
        name: &str,
        config: InterfaceConfig,
    ) -> Result<(), NetServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::SetInterfaceConfigQueryParameters {
            name: name_buffer,
            config,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::SetInterfaceConfigQueryParameters::HANDLE_NAME_MOBJ] =
            name_mobj.into_handle();

        self.ipc_client.call::<
            messages::Type,
            messages::SetInterfaceConfigQueryParameters,
            messages::SetInterfaceConfigReply,
            messages::NetServerError,
        >(messages::Type::SetInterfaceConfig, query, query_handles)?;

        Ok(())
    }

    /// Get interface configuration
    pub fn get_interface_config(&self, name: &str) -> Result<InterfaceConfig, NetServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::GetInterfaceConfigQueryParameters { name: name_buffer };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::GetInterfaceConfigQueryParameters::HANDLE_NAME_MOBJ] =
            name_mobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::GetInterfaceConfigQueryParameters,
            messages::GetInterfaceConfigReply,
            messages::NetServerError,
        >(messages::Type::GetInterfaceConfig, query, query_handles)?;

        Ok(reply.config)
    }

    /// Get information on the interface
    pub fn get_interface_info(&self, name: &str) -> Result<InterfaceInfo, NetServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::GetInterfaceInfoQueryParameters { name: name_buffer };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::GetInterfaceInfoQueryParameters::HANDLE_NAME_MOBJ] =
            name_mobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::GetInterfaceInfoQueryParameters,
            messages::GetInterfaceInfoReply,
            messages::NetServerError,
        >(messages::Type::GetInterfaceInfo, query, query_handles)?;

        Ok(reply.info)
    }

    /// Create or overwrite (on prefix+iface) a route
    pub fn set_route(
        &self,
        prefix: IpPrefix,
        gateway: Option<IpAddress>,
        iface: &str,
        metric: usize,
    ) -> Result<(), NetServerCallError> {
        let (iface_mobj, iface_buffer) = ipc::Buffer::new_local(iface.as_bytes()).into_shared();

        let query = messages::SetRouteQueryParameters {
            prefix,
            gateway,
            iface: iface_buffer,
            metric,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::SetRouteQueryParameters::HANDLE_IFACE_MOBJ] =
            iface_mobj.into_handle();

        self.ipc_client.call::<
            messages::Type,
            messages::SetRouteQueryParameters,
            messages::SetRouteReply,
            messages::NetServerError,
        >(messages::Type::SetRoute, query, query_handles)?;

        Ok(())
    }

    /// Remove a route
    pub fn remove_route(&self, prefix: IpPrefix, iface: &str) -> Result<(), NetServerCallError> {
        let (iface_mobj, iface_buffer) = ipc::Buffer::new_local(iface.as_bytes()).into_shared();

        let query = messages::RemoveRouteQueryParameters {
            prefix,
            iface: iface_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::RemoveRouteQueryParameters::HANDLE_IFACE_MOBJ] =
            iface_mobj.into_handle();

        self.ipc_client.call::<
            messages::Type,
            messages::RemoveRouteQueryParameters,
            messages::RemoveRouteReply,
            messages::NetServerError,
        >(messages::Type::RemoveRoute, query, query_handles)?;

        Ok(())
    }

    /// call ipc ListRoutes
    pub fn list_routes(&self) -> Result<Vec<Route>, NetServerCallError> {
        // We don't know how many routes there are, so we start with a small buffer and grow it until it's big enough.
        let mut size = 256;

        let allocated_buffer = loop {
            let mut allocated_buffer = {
                let mut vec = Vec::with_capacity(size);
                unsafe { vec.set_len(size) };
                vec
            };

            let (buffer_mobj, buffer) = ipc::Buffer::new_local(&allocated_buffer).into_shared();

            let query = messages::ListRoutesQueryParameters { buffer };

            let mut query_handles = ipc::KHandles::new();
            query_handles[messages::ListRoutesQueryParameters::HANDLE_BUFFER_MOBJ] =
                buffer_mobj.into_handle();

            let res = self.ipc_client.call::<messages::Type, messages::ListRoutesQueryParameters, messages::ListRoutesReply, messages::NetServerError>(
                messages::Type::ListRoutes,
                query,
                query_handles,
            );

            if let Err(ipc::CallError::ReplyError(messages::NetServerError::BufferTooSmall)) = res {
                size *= 2;
                continue;
            }

            let (reply, _reply_handles) = res?;

            unsafe { allocated_buffer.set_len(reply.buffer_used_len) };
            break allocated_buffer;
        };

        let routes =
            RoutesBlock::read(&allocated_buffer).expect("Failed to read routes list from buffer");

        Ok(routes)
    }
}
