use alloc::vec::Vec;

use super::{PciDeviceInfo, info_block::InfoBlock, messages};
use crate::{
    drivers::pci::types::{PciAddress, PciHeader},
    ipc::{self, Handle},
    kobject::KObject,
};

pub type PciServerCallError = ipc::CallError<messages::PciServerError>;

/// Low level PCI client implementation.
#[derive(Debug)]
pub struct Client {
    ipc_client: ipc::Client<'static>,
}

impl Client {
    /// Creates a new PCI client.
    pub fn new() -> Self {
        Self {
            ipc_client: ipc::Client::new(messages::PORT_NAME, messages::VERSION),
        }
    }

    /// List all PCI devices that match the given criteria.
    pub fn list(
        &self,
        vendor_id: Option<u16>,
        device_id: Option<u16>,
        class: Option<u8>,
        subclass: Option<u8>,
    ) -> Result<Vec<PciDeviceInfo>, PciServerCallError> {
        // We don't know how many devices there are, so we start with a small buffer and grow it until it's big enough.
        let mut size = 256;

        let allocated_buffer = loop {
            let mut allocated_buffer = {
                let mut vec = Vec::with_capacity(size);
                unsafe { vec.set_len(size) };
                vec
            };

            let (buffer_mobj, buffer) = ipc::Buffer::new_local(&allocated_buffer).into_shared();

            let query = messages::ListQueryParameters {
                vendor_id,
                device_id,
                class,
                subclass,
                buffer,
            };

            let mut query_handles = ipc::KHandles::new();
            query_handles[messages::ListQueryParameters::HANDLE_BUFFER_MOBJ] =
                buffer_mobj.into_handle();

            let res = self.ipc_client.call::<
                messages::Type,
                messages::ListQueryParameters,
                messages::ListReply,
                messages::PciServerError,
            >(messages::Type::List, query, query_handles);

            if let Err(ipc::CallError::ReplyError(messages::PciServerError::InvalidArgument)) = res
            {
                // Buffer too small, try again with a larger buffer
                size *= 2;
                continue;
            }

            let (reply, _reply_handles) = res?;

            unsafe { allocated_buffer.set_len(reply.buffer_used_len) };
            break allocated_buffer;
        };

        let devices = InfoBlock::read(&allocated_buffer)
            .expect("Failed to read PCI devices block from buffer");

        Ok(devices)
    }

    /// Get device information for the PCI device at the given address.
    /// Returns None if no device is found at that address.
    pub fn get_by_address(&self, address: PciAddress) -> Result<PciDeviceInfo, PciServerCallError> {
        let query = messages::GetByAddressQueryParameters { address };
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::GetByAddressQueryParameters,
            messages::GetByAddressReply,
            messages::PciServerError,
        >(
            messages::Type::GetByAddress,
            query,
            query_handles,
        )?;

        Ok(reply.device_info)
    }

    /// Open a handle to the PCI device at the given address.
    pub fn open(&self, address: PciAddress) -> Result<Handle, PciServerCallError> {
        let query = messages::OpenQueryParameters { address };

        let (reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::OpenQueryParameters,
            messages::OpenReply,
            messages::PciServerError,
        >(messages::Type::Open, query, ipc::KHandles::new())?;

        Ok(reply.handle)
    }

    /// Close a handle to a PCI device.
    pub fn close(&self, handle: Handle) -> Result<(), PciServerCallError> {
        let query = messages::CloseQueryParameters { handle };

        self.ipc_client.call::<
            messages::Type,
            messages::CloseQueryParameters,
            messages::CloseReply,
            messages::PciServerError,
        >(messages::Type::Close, query, ipc::KHandles::new())?;

        Ok(())
    }

    /// Get the PCI header for a device.
    pub fn get_header(&self, handle: Handle) -> Result<PciHeader, PciServerCallError> {
        use core::mem::MaybeUninit;

        // Create a buffer to receive the header
        let mut header_storage: MaybeUninit<PciHeader> = MaybeUninit::uninit();
        let header_bytes = unsafe {
            core::slice::from_raw_parts_mut(
                header_storage.as_mut_ptr() as *mut u8,
                core::mem::size_of::<PciHeader>(),
            )
        };

        let (buffer_mobj, buffer) = ipc::Buffer::new_local(header_bytes).into_shared();

        let query = messages::GetHeaderQueryParameters {
            handle,
            header_buffer: buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::GetHeaderQueryParameters::HANDLE_HEADER_BUFFER_MOBJ] =
            buffer_mobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<
            messages::Type,
            messages::GetHeaderQueryParameters,
            messages::GetHeaderReply,
            messages::PciServerError,
        >(messages::Type::GetHeader, query, query_handles)?;

        // Safety: The server has written a valid PciHeader into the buffer
        Ok(unsafe { header_storage.assume_init() })
    }

    /// Enable or disable a device.
    pub fn enable(
        &self,
        handle: Handle,
        memory: bool,
        io: bool,
        bus_master: bool,
    ) -> Result<(), PciServerCallError> {
        let query = messages::EnableQueryParameters {
            handle,
            memory,
            io,
            bus_master,
        };

        self.ipc_client.call::<
            messages::Type,
            messages::EnableQueryParameters,
            messages::EnableReply,
            messages::PciServerError,
        >(messages::Type::Enable, query, ipc::KHandles::new())?;

        Ok(())
    }
}
