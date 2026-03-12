use super::messages;
use crate::{
    drivers::pci::PciAddress,
    ipc,
    kobject::{self, KObject},
    net::types::MacAddress,
    service,
};
use alloc::sync::Arc;

/// Net device server interface
pub trait NetDeviceServer {
    type Error: Into<messages::NetDeviceError>;

    /// Create a new network device from a PCI address.
    fn create(
        &self,
        sender_id: u64,
        name: &str,
        pci_address: PciAddress,
    ) -> Result<ipc::Handle, Self::Error>;

    /// Destroy a network device.
    fn destroy(&self, sender_id: u64, handle: ipc::Handle) -> Result<(), Self::Error>;

    /// Get the link status of a network device.
    fn get_link_status(&self, sender_id: u64, handle: ipc::Handle) -> Result<bool, Self::Error>;

    /// Set the port for link status change notifications.
    /// Pass Some(port) to register for notifications, or None to unregister.
    fn set_link_status_change_port(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
        port: Option<kobject::PortSender>,
        correlation: u64,
    ) -> Result<(), Self::Error>;

    /// Get the MAC address of a network device.
    fn get_mac_address(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
    ) -> Result<MacAddress, Self::Error>;
}

/// The main server structure
#[derive(Debug)]
pub struct Server<Impl: NetDeviceServer + 'static> {
    inner: Impl,
}

impl<Impl: NetDeviceServer + 'static> Server<Impl> {
    pub fn new(inner: Impl) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub fn setup_ipc_server(
        self: &Arc<Self>,
        port_name: &'static str,
        runner: &service::Runner,
    ) -> Result<(), kobject::Error> {
        let builder = ipc::ManagedServerBuilder::<
            _,
            messages::NetDeviceError,
            messages::NetDeviceError,
        >::new(&self, port_name, messages::VERSION);

        let builder = builder.with_handler(messages::Type::Create, Self::create_handler);
        let builder = builder.with_handler(messages::Type::Destroy, Self::destroy_handler);
        let builder =
            builder.with_handler(messages::Type::GetLinkStatus, Self::get_link_status_handler);
        let builder = builder.with_handler(
            messages::Type::SetLinkStatusChangePort,
            Self::set_link_status_change_port_handler,
        );
        let builder =
            builder.with_handler(messages::Type::GetMacAddress, Self::get_mac_address_handler);

        runner.add_component(Arc::new(builder.build()?));

        Ok(())
    }

    fn create_handler(
        &self,
        query: messages::CreateQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CreateReply, ipc::KHandles), messages::NetDeviceError> {
        let name_view = {
            let handle = query_handles.take(messages::CreateQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .map_err(|_| messages::NetDeviceError::InvalidArgument)?
        };

        let name = unsafe { name_view.str() };

        let handle = self
            .inner
            .create(sender_id, name, query.pci_address)
            .map_err(Into::into)?;

        Ok((messages::CreateReply { handle }, ipc::KHandles::new()))
    }

    fn destroy_handler(
        &self,
        query: messages::DestroyQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::DestroyReply, ipc::KHandles), messages::NetDeviceError> {
        self.inner
            .destroy(sender_id, query.handle)
            .map_err(Into::into)?;

        Ok((messages::DestroyReply {}, ipc::KHandles::new()))
    }

    fn get_link_status_handler(
        &self,
        query: messages::GetLinkStatusQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetLinkStatusReply, ipc::KHandles), messages::NetDeviceError> {
        let link_up = self
            .inner
            .get_link_status(sender_id, query.handle)
            .map_err(Into::into)?;

        Ok((
            messages::GetLinkStatusReply { link_up },
            ipc::KHandles::new(),
        ))
    }

    fn set_link_status_change_port_handler(
        &self,
        query: messages::SetLinkStatusChangePortQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::SetLinkStatusChangePortReply, ipc::KHandles), messages::NetDeviceError>
    {
        let handle =
            query_handles.take(messages::SetLinkStatusChangePortQueryParameters::HANDLE_PORT);

        let port = if handle.valid() {
            Some(
                kobject::PortSender::from_handle(handle)
                    .map_err(|_| messages::NetDeviceError::InvalidArgument)?,
            )
        } else {
            None
        };

        self.inner
            .set_link_status_change_port(sender_id, query.handle, port, query.correlation)
            .map_err(Into::into)?;

        Ok((
            messages::SetLinkStatusChangePortReply {
                registration_handle: ipc::Handle::INVALID,
            },
            ipc::KHandles::new(),
        ))
    }

    fn get_mac_address_handler(
        &self,
        query: messages::GetMacAddressQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetMacAddressReply, ipc::KHandles), messages::NetDeviceError> {
        let mac_address = self
            .inner
            .get_mac_address(sender_id, query.handle)
            .map_err(Into::into)?;

        Ok((
            messages::GetMacAddressReply { mac_address },
            ipc::KHandles::new(),
        ))
    }
}
