use core::fmt;

use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use log::error;

use crate::{drivers::pci::PciAddress, ipc, kobject};

use super::messages;

pub use messages::NetError;

/// Net server interface that must be implemented by the net server.
#[async_trait]
pub trait NetServer: Send + Sync {
    type Error: Into<NetError>;

    /// Create a new network device.
    async fn create_device(
        &self,
        sender_id: u64,
        name: &str,
        driver_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<(), Self::Error>;

    /// Destroy a network device.
    async fn destroy_device(&self, sender_id: u64, name: &str) -> Result<(), Self::Error>;
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
        port_name: &'static str,
    ) -> Result<ipc::AsyncServer, kobject::Error> {
        let builder = ipc::ManagedAsyncServerBuilder::<_, NetError, NetError>::new(
            &self,
            port_name,
            messages::VERSION,
        );

        let builder =
            builder.with_handler(messages::Type::CreateDevice, Self::create_device_handler);
        let builder =
            builder.with_handler(messages::Type::DestroyDevice, Self::destroy_device_handler);

        builder.build()
    }

    async fn create_device_handler(
        self: Arc<Self>,
        query: messages::CreateDeviceQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CreateDeviceReply, ipc::KHandles), NetError> {
        let name_view = {
            let handle =
                query_handles.take(messages::CreateDeviceQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer view")?
        };

        let driver_port_name_view = {
            let handle = query_handles
                .take(messages::CreateDeviceQueryParameters::HANDLE_DRIVER_PORT_NAME_MOBJ);
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
            .create_device(sender_id, name, driver_port_name, query.pci_address)
            .await
            .map_err(Into::into)?;

        Ok((messages::CreateDeviceReply {}, ipc::KHandles::new()))
    }

    async fn destroy_device_handler(
        self: Arc<Self>,
        query: messages::DestroyDeviceQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::DestroyDeviceReply, ipc::KHandles), NetError> {
        let name_view = {
            let handle =
                query_handles.take(messages::DestroyDeviceQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer view")?
        };

        let name = unsafe { name_view.str() };

        self.inner
            .destroy_device(sender_id, name)
            .await
            .map_err(Into::into)?;

        Ok((messages::DestroyDeviceReply {}, ipc::KHandles::new()))
    }
}

/// Extension trait for Result to add context
trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, NetError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, NetError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            NetError::InvalidArgument
        })
    }
}
