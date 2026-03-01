use alloc::{sync::Arc, vec::Vec};

use crate::{ipc, kobject};

use super::{PciDeviceInfo, info_block::InfoBlock, messages};
use crate::drivers::pci::types::PciAddress;

pub use messages::PciServerError;

/// PCI server interface
pub trait PciServer {
    type Error: Into<PciServerError>;

    /// List all PCI devices that match the given class and optional subclass.
    fn list_by_class(
        &self,
        sender_id: u64,
        class: u8,
        subclass: Option<u8>,
    ) -> Result<Vec<PciDeviceInfo>, Self::Error>;

    /// List all PCI devices that match the given vendor ID and optional device ID.
    fn list_by_device_id(
        &self,
        sender_id: u64,
        vendor_id: u16,
        device_id: Option<u16>,
    ) -> Result<Vec<PciDeviceInfo>, Self::Error>;

    /// Get device information for the PCI device at the given address.
    fn get_by_address(
        &self,
        sender_id: u64,
        address: PciAddress,
    ) -> Result<PciDeviceInfo, Self::Error>;

    /// Open a handle to the PCI device at the given address.
    fn open(&self, sender_id: u64, address: PciAddress) -> Result<ipc::Handle, Self::Error>;

    /// Close a handle to a PCI device.
    fn close(&self, sender_id: u64, handle: ipc::Handle) -> Result<(), Self::Error>;
}

/// The main server structure
#[derive(Debug)]
pub struct Server<Impl: PciServer + 'static> {
    inner: Impl,
}

impl<Impl: PciServer + 'static> Server<Impl> {
    pub fn new(inner: Impl) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::Server, kobject::Error> {
        let builder = ipc::ManagedServerBuilder::<_, PciServerError, PciServerError>::new(
            &self,
            messages::PORT_NAME,
            messages::VERSION,
        );

        let builder =
            builder.with_handler(messages::Type::ListByClass, Self::list_by_class_handler);
        let builder = builder.with_handler(
            messages::Type::ListByDeviceId,
            Self::list_by_device_id_handler,
        );
        let builder =
            builder.with_handler(messages::Type::GetByAddress, Self::get_by_address_handler);
        let builder = builder.with_handler(messages::Type::Open, Self::open_handler);
        let builder = builder.with_handler(messages::Type::Close, Self::close_handler);

        builder.build()
    }

    fn list_by_class_handler(
        &self,
        query: messages::ListByClassQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListByClassReply, ipc::KHandles), PciServerError> {
        let devices = self
            .inner
            .list_by_class(sender_id, query.class, query.subclass)
            .map_err(Into::into)?;

        let mut buffer_view = {
            let handle =
                query_handles.take(messages::ListByClassQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create buffer view")?
        };

        let buffer = buffer_view.buffer_mut();
        let result = InfoBlock::build(&devices, buffer);

        let buffer_used_len = match result {
            Ok(size) => size,
            Err(_required_size) => {
                return Err(PciServerError::InvalidArgument);
            }
        };

        Ok((
            messages::ListByClassReply { buffer_used_len },
            ipc::KHandles::new(),
        ))
    }

    fn list_by_device_id_handler(
        &self,
        query: messages::ListByDeviceIdQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListByDeviceIdReply, ipc::KHandles), PciServerError> {
        let devices = self
            .inner
            .list_by_device_id(sender_id, query.vendor_id, query.device_id)
            .map_err(Into::into)?;

        let mut buffer_view = {
            let handle =
                query_handles.take(messages::ListByDeviceIdQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create buffer view")?
        };

        let buffer = buffer_view.buffer_mut();
        let result = InfoBlock::build(&devices, buffer);

        let buffer_used_len = match result {
            Ok(size) => size,
            Err(_required_size) => {
                return Err(PciServerError::InvalidArgument);
            }
        };

        Ok((
            messages::ListByDeviceIdReply { buffer_used_len },
            ipc::KHandles::new(),
        ))
    }

    fn get_by_address_handler(
        &self,
        query: messages::GetByAddressQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetByAddressReply, ipc::KHandles), PciServerError> {
        let device_info = self
            .inner
            .get_by_address(sender_id, query.address)
            .map_err(Into::into)?;

        Ok((
            messages::GetByAddressReply { device_info },
            ipc::KHandles::new(),
        ))
    }

    fn open_handler(
        &self,
        query: messages::OpenQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::OpenReply, ipc::KHandles), PciServerError> {
        let handle = self
            .inner
            .open(sender_id, query.address)
            .map_err(Into::into)?;

        Ok((messages::OpenReply { handle }, ipc::KHandles::new()))
    }

    fn close_handler(
        &self,
        query: messages::CloseQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CloseReply, ipc::KHandles), PciServerError> {
        self.inner
            .close(sender_id, query.handle)
            .map_err(Into::into)?;

        Ok((messages::CloseReply {}, ipc::KHandles::new()))
    }
}

trait ResultExt<T> {
    fn invalid_arg(self, msg: &str) -> Result<T, PciServerError>;
}

impl<T> ResultExt<T> for Result<T, kobject::Error> {
    fn invalid_arg(self, msg: &str) -> Result<T, PciServerError> {
        self.map_err(|e| {
            log::error!("{}: {}", msg, e);
            PciServerError::InvalidArgument
        })
    }
}
