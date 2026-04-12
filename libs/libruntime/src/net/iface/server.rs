use core::fmt;

use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use log::error;

use crate::{drivers::pci::PciAddress, ipc, kobject};

use super::messages;

pub use messages::NetServerError;

/// Net server interface that must be implemented by the net server.
#[async_trait]
pub trait NetServer: Send + Sync {
    type Error: Into<NetServerError>;

    async fn process_terminated(&self, _pid: u64) {}

    /// Create a new network interface.
    async fn create_interface(
        &self,
        sender_id: u64,
        name: &str,
        driver_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<(), Self::Error>;

    /// Destroy a network interface.
    async fn destroy_interface(&self, sender_id: u64, name: &str) -> Result<(), Self::Error>;
}

/// The main server structure.
#[derive(Debug)]
pub struct Server<Impl: NetServer + 'static> {
    inner: Impl,
}

impl<Impl: NetServer + 'static> Server<Impl> {
    pub fn new(inner: Impl) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub fn build_ipc_server(
        self: &Arc<Self>,
    ) -> Result<(ipc::AsyncServer, ipc::AsyncProcessTerminationListener), kobject::Error> {
        let builder = ipc::ManagedAsyncServerBuilder::<_, NetServerError, NetServerError>::new(
            &self,
            messages::PORT_NAME,
            messages::VERSION,
        );

        let builder = builder.with_handler(
            messages::Type::CreateInterface,
            Self::create_interface_handler,
        );
        let builder = builder.with_handler(
            messages::Type::DestroyInterface,
            Self::destroy_interface_handler,
        );

        let listener = ipc::AsyncProcessTerminationListener::from_handler_method(
            self,
            Self::process_terminated_handler,
        )?;
        let server = builder.build()?;

        Ok((server, listener))
    }

    async fn create_interface_handler(
        self: Arc<Self>,
        query: messages::CreateInterfaceQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CreateInterfaceReply, ipc::KHandles), NetServerError> {
        let name_view = {
            let handle =
                query_handles.take(messages::CreateInterfaceQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer view")?
        };

        let driver_port_name_view = {
            let handle = query_handles
                .take(messages::CreateInterfaceQueryParameters::HANDLE_DRIVER_PORT_NAME_MOBJ);
            ipc::BufferView::new(
                handle,
                &query.driver_port_name,
                ipc::BufferViewAccess::ReadOnly,
            )
            .invalid_arg("Failed to create driver port name buffer view")?
        };

        let name = unsafe { name_view.str() };
        let driver_port_name = unsafe { driver_port_name_view.str() };

        self.inner
            .create_interface(sender_id, name, driver_port_name, query.pci_address)
            .await
            .map_err(Into::into)?;

        Ok((messages::CreateInterfaceReply {}, ipc::KHandles::new()))
    }

    async fn destroy_interface_handler(
        self: Arc<Self>,
        query: messages::DestroyInterfaceQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::DestroyInterfaceReply, ipc::KHandles), NetServerError> {
        let name_view = {
            let handle =
                query_handles.take(messages::DestroyInterfaceQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer view")?
        };

        let name = unsafe { name_view.str() };

        self.inner
            .destroy_interface(sender_id, name)
            .await
            .map_err(Into::into)?;

        Ok((messages::DestroyInterfaceReply {}, ipc::KHandles::new()))
    }

    async fn process_terminated_handler(self: Arc<Self>, pid: u64) {
        self.inner.process_terminated(pid).await;
    }
}

/// Extension trait for Result to add context
trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, NetServerError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, NetServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            NetServerError::InvalidArgument
        })
    }
}
