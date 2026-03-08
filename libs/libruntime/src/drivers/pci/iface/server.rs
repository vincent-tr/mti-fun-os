use alloc::{sync::Arc, vec::Vec};

use crate::{ipc, kobject};

use super::{
    CapabilityInfo, EnableMsiData, PciDeviceInfo, capability_block::CapabilityBlock,
    info_block::InfoBlock, messages,
};
use crate::drivers::pci::types::{PciAddress, PciHeader};

pub use messages::PciServerError;

/// PCI server interface
pub trait PciServer {
    type Error: Into<PciServerError>;

    /// List all PCI devices that match the criteria.
    fn list(
        &self,
        sender_id: u64,
        vendor_id: Option<u16>,
        device_id: Option<u16>,
        class: Option<u8>,
        subclass: Option<u8>,
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

    /// Get the PCI header for a device.
    fn get_header(&self, sender_id: u64, handle: ipc::Handle) -> Result<PciHeader, Self::Error>;

    /// Enable or disable memory, I/O, and bus mastering for a device.
    fn enable(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
        memory: bool,
        io: bool,
        bus_master: bool,
    ) -> Result<(), Self::Error>;

    /// List all capabilities for a device.
    fn list_capabilities(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
    ) -> Result<Vec<CapabilityInfo>, Self::Error>;

    /// Read capability data from a device.
    fn read_capability(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
        capability_index: usize,
        offset: usize,
        data: &mut [u8],
    ) -> Result<(), Self::Error>;

    /// Write capability data to a device.
    fn write_capability(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
        capability_index: usize,
        offset: usize,
        data: &[u8],
    ) -> Result<(), Self::Error>;

    /// Enable or disable MSI for a device.
    fn enable_msi(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
        enable: EnableMsiData,
    ) -> Result<(), Self::Error>;
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

        let builder = builder.with_handler(messages::Type::List, Self::list_handler);
        let builder =
            builder.with_handler(messages::Type::GetByAddress, Self::get_by_address_handler);
        let builder = builder.with_handler(messages::Type::Open, Self::open_handler);
        let builder = builder.with_handler(messages::Type::Close, Self::close_handler);
        let builder = builder.with_handler(messages::Type::GetHeader, Self::get_header_handler);
        let builder = builder.with_handler(messages::Type::Enable, Self::enable_handler);
        let builder = builder.with_handler(
            messages::Type::ListCapabilities,
            Self::list_capabilities_handler,
        );
        let builder = builder.with_handler(
            messages::Type::ReadCapability,
            Self::read_capability_handler,
        );
        let builder = builder.with_handler(
            messages::Type::WriteCapability,
            Self::write_capability_handler,
        );
        let builder = builder.with_handler(messages::Type::EnableMsi, Self::enable_msi_handler);

        builder.build()
    }

    fn list_handler(
        &self,
        query: messages::ListQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListReply, ipc::KHandles), PciServerError> {
        let devices = self
            .inner
            .list(
                sender_id,
                query.vendor_id,
                query.device_id,
                query.class,
                query.subclass,
            )
            .map_err(Into::into)?;

        let mut buffer_view = {
            let handle = query_handles.take(messages::ListQueryParameters::HANDLE_BUFFER_MOBJ);
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
            messages::ListReply { buffer_used_len },
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

    fn get_header_handler(
        &self,
        query: messages::GetHeaderQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetHeaderReply, ipc::KHandles), PciServerError> {
        let header = self
            .inner
            .get_header(sender_id, query.handle)
            .map_err(Into::into)?;

        // Get the buffer view from the query
        let mut buffer_view = {
            let handle =
                query_handles.take(messages::GetHeaderQueryParameters::HANDLE_HEADER_BUFFER_MOBJ);
            ipc::BufferView::new(
                handle,
                &query.header_buffer,
                ipc::BufferViewAccess::ReadWrite,
            )
            .invalid_arg("Failed to create buffer view")?
        };

        let buffer = buffer_view.buffer_mut();

        // Check buffer size
        if buffer.len() < core::mem::size_of::<PciHeader>() {
            return Err(PciServerError::InvalidArgument);
        }

        // Write header to buffer
        unsafe {
            core::ptr::write(buffer.as_mut_ptr() as *mut PciHeader, header);
        }

        Ok((messages::GetHeaderReply {}, ipc::KHandles::new()))
    }

    fn enable_handler(
        &self,
        query: messages::EnableQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::EnableReply, ipc::KHandles), PciServerError> {
        self.inner
            .enable(
                sender_id,
                query.handle,
                query.memory,
                query.io,
                query.bus_master,
            )
            .map_err(Into::into)?;

        Ok((messages::EnableReply {}, ipc::KHandles::new()))
    }

    fn list_capabilities_handler(
        &self,
        query: messages::ListCapabilitiesQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListCapabilitiesReply, ipc::KHandles), PciServerError> {
        let capabilities = self
            .inner
            .list_capabilities(sender_id, query.handle)
            .map_err(Into::into)?;

        let mut buffer_view = {
            let handle = query_handles
                .take(messages::ListCapabilitiesQueryParameters::HANDLE_CAPABILITIES_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create buffer view")?
        };

        let buffer = buffer_view.buffer_mut();
        let result = CapabilityBlock::build(&capabilities, buffer);

        let buffer_used_len = match result {
            Ok(size) => size,
            Err(_required_size) => {
                return Err(PciServerError::InvalidArgument);
            }
        };

        Ok((
            messages::ListCapabilitiesReply { buffer_used_len },
            ipc::KHandles::new(),
        ))
    }

    fn read_capability_handler(
        &self,
        query: messages::ReadCapabilityQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ReadCapabilityReply, ipc::KHandles), PciServerError> {
        let mut buffer_view = {
            let handle = query_handles
                .take(messages::ReadCapabilityQueryParameters::HANDLE_CAPABILITY_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create buffer view")?
        };

        let buffer = buffer_view.buffer_mut();

        // Check buffer size matches query size
        if buffer.len() != query.size {
            return Err(PciServerError::InvalidArgument);
        }

        self.inner
            .read_capability(
                sender_id,
                query.handle,
                query.capability_index,
                query.offset,
                buffer,
            )
            .map_err(Into::into)?;

        Ok((messages::ReadCapabilityReply {}, ipc::KHandles::new()))
    }

    fn write_capability_handler(
        &self,
        query: messages::WriteCapabilityQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::WriteCapabilityReply, ipc::KHandles), PciServerError> {
        let buffer_view = {
            let handle = query_handles
                .take(messages::WriteCapabilityQueryParameters::HANDLE_CAPABILITY_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create buffer view")?
        };

        let buffer = buffer_view.buffer();

        // Check buffer size matches query size
        if buffer.len() != query.size {
            return Err(PciServerError::InvalidArgument);
        }

        self.inner
            .write_capability(
                sender_id,
                query.handle,
                query.capability_index,
                query.offset,
                buffer,
            )
            .map_err(Into::into)?;

        Ok((messages::WriteCapabilityReply {}, ipc::KHandles::new()))
    }

    fn enable_msi_handler(
        &self,
        query: messages::EnableMsiQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::EnableMsiReply, ipc::KHandles), PciServerError> {
        self.inner
            .enable_msi(sender_id, query.handle, query.enable)
            .map_err(Into::into)?;

        Ok((messages::EnableMsiReply {}, ipc::KHandles::new()))
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
