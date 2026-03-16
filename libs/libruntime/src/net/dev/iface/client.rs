use core::pin::Pin;

use alloc::boxed::Box;

use super::messages;
use crate::{
    drivers::pci::PciAddress,
    ipc,
    kobject::{self, KObject},
    net::types::{BufferPool, MacAddress},
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

    /// Get the name of the server port this client is connected to.
    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    /// Create a new network device.
    pub fn create(
        &self,
        name: &str,
        pci_address: PciAddress,
        buffer_pool: &BufferPool,
    ) -> Result<ipc::Handle, NetDeviceServerCallError> {
        let (name_memobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::CreateQueryParameters {
            name: name_buffer,
            pci_address,
            net_buffer_pool: messages::NetBufferPoolConfig {
                buffer_count: buffer_pool.buffer_count,
                buffer_size: buffer_pool.buffer_size,
            },
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::CreateQueryParameters::HANDLE_NAME_MOBJ] =
            name_memobj.into_handle();
        query_handles[messages::CreateQueryParameters::HANDLE_NET_BUFFER_POOL_MOBJ] =
            buffer_pool.mobj.clone().into_handle();

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

    /// Set the port for link status change notifications.
    /// Pass Some(port) to register for notifications, or None to unregister.
    pub fn set_link_status_change_port(
        &self,
        handle: ipc::Handle,
        port: Option<kobject::PortSender>,
        correlation: u64,
    ) -> Result<(), NetDeviceServerCallError> {
        let query = messages::SetLinkStatusChangePortQueryParameters {
            handle,
            correlation,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::SetLinkStatusChangePortQueryParameters::HANDLE_PORT] =
            if let Some(port) = port {
                port.into_handle()
            } else {
                // Use invalid handle to unset the port
                kobject::Handle::invalid()
            };

        let (_reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::SetLinkStatusChangePortQueryParameters,
            messages::SetLinkStatusChangePortReply,
            messages::NetDeviceError,
        >(
            messages::Type::SetLinkStatusChangePort,
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

    /// Transmit data on a network device.
    /// Returns the number of descriptors that were accepted for transmission.
    ///
    /// Note: the IPC message is designed to take up to 13 descriptors at a time, so if more than 13 descriptors are passed, only the first 13 will be transmitted.
    pub fn tx(
        &self,
        handle: ipc::Handle,
        descriptors: &[messages::TxBufferDescriptor],
    ) -> Result<usize, NetDeviceServerCallError> {
        assert!(
            descriptors.len() <= messages::TxQueryParameters::DESCRIPTOR_COUNT,
            "Too many descriptors passed to tx: {} (max {})",
            descriptors.len(),
            messages::TxQueryParameters::DESCRIPTOR_COUNT
        );

        let mut tx_descriptors = [messages::TxBufferDescriptor::default();
            messages::TxQueryParameters::DESCRIPTOR_COUNT];
        let count = descriptors.len();
        tx_descriptors[..count].copy_from_slice(&descriptors[..count]);

        let query = messages::TxQueryParameters {
            handle,
            tx_descriptors,
        };
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::TxQueryParameters,
            messages::TxReply,
            messages::NetDeviceError,
        >(messages::Type::Tx, query, query_handles)?;

        Ok(reply.sent_buffers)
    }

    /// Set the port for Tx free notifications.
    /// Pass Some(port) to register for notifications, or None to unregister.
    pub fn set_tx_free_port(
        &self,
        handle: ipc::Handle,
        port: Option<kobject::PortSender>,
        correlation: u64,
    ) -> Result<(), NetDeviceServerCallError> {
        let query = messages::SetTxFreePortQueryParameters {
            handle,
            correlation,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::SetTxFreePortQueryParameters::HANDLE_PORT] =
            if let Some(port) = port {
                port.into_handle()
            } else {
                kobject::Handle::invalid()
            };

        let (_reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::SetTxFreePortQueryParameters,
            messages::SetTxFreePortReply,
            messages::NetDeviceError,
        >(messages::Type::SetTxFreePort, query, query_handles)?;

        Ok(())
    }

    /// Add receive buffers to a network device.
    /// Returns the number of buffers that were successfully added.
    ///
    /// Note: the IPC message is designed to take up to 26 buffers at a time, so if more than 26 buffers are passed, only the first 26 will be added.
    pub fn add_rx_buffers(
        &self,
        handle: ipc::Handle,
        buffer_indexes: &[u32],
    ) -> Result<usize, NetDeviceServerCallError> {
        assert!(
            buffer_indexes.len() <= messages::AddRxBuffersQueryParameters::BUFFER_COUNT,
            "Too many buffers passed to add_rx_buffers: {} (max {})",
            buffer_indexes.len(),
            messages::AddRxBuffersQueryParameters::BUFFER_COUNT
        );

        let mut buffers =
            [BufferPool::INVALID_INDEX; messages::AddRxBuffersQueryParameters::BUFFER_COUNT];
        let count = buffer_indexes.len();
        buffers[..count].copy_from_slice(&buffer_indexes[..count]);

        let query = messages::AddRxBuffersQueryParameters { handle, buffers };
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::AddRxBuffersQueryParameters,
            messages::AddRxBuffersReply,
            messages::NetDeviceError,
        >(messages::Type::AddRxBuffers, query, query_handles)?;

        Ok(reply.added_buffers)
    }

    /// Set the port for Rx arrived notifications.
    /// Pass Some(port) to register for notifications, or None to unregister.
    pub fn set_rx_port(
        &self,
        handle: ipc::Handle,
        port: Option<kobject::PortSender>,
        correlation: u64,
    ) -> Result<(), NetDeviceServerCallError> {
        let query = messages::SetRxPortQueryParameters {
            handle,
            correlation,
        };

        let mut query_handles = ipc::KHandles::new();
        if let Some(port) = port {
            query_handles[messages::SetRxPortQueryParameters::HANDLE_PORT] = port.into_handle();
        } else {
            query_handles[messages::SetRxPortQueryParameters::HANDLE_PORT] =
                kobject::Handle::invalid();
        }

        let (_reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::SetRxPortQueryParameters,
            messages::SetRxPortReply,
            messages::NetDeviceError,
        >(messages::Type::SetRxPort, query, query_handles)?;

        Ok(())
    }
}
