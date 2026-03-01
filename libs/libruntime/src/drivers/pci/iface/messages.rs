use core::fmt;

use crate::{
    drivers::pci::types::PciAddress,
    ipc::{Handle, buffer_messages::Buffer},
};

/// Name of the IPC port for the PCI server.
pub const PORT_NAME: &str = "pci-server";

/// Version of the PCI interface management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in PCI interface management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    // Discovery messages
    ListByClass,
    ListByDeviceId,
    GetByAddress,

    // Handles management messages
    Open,
    Close,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}

/// Errors used by the PCI server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum PciServerError {
    InvalidArgument = 1,
    RuntimeError,
}

impl fmt::Display for PciServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
        }
    }
}

/// Parameters for the ListByClass message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListByClassQueryParameters {
    /// The class code to query for (8 bits).
    pub class: u8,

    /// The subclass code to query for (8 bits).
    pub subclass: Option<u8>,

    /// Buffer to write the list of devices info into.
    pub buffer: Buffer,
}

impl ListByClassQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 1;
}

/// Reply for the ListByClass message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListByClassReply {
    /// Number of bytes used in the buffer to write the list of devices info (if the call succeeds)
    pub buffer_used_len: usize,
}

/// Parameters for the ListByDeviceId message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListByDeviceIdQueryParameters {
    /// The vendor ID to query for (16 bits).
    pub vendor_id: u16,

    /// The device ID to query for (16 bits).
    pub device_id: Option<u16>,

    /// Buffer to write the list of devices info into.
    pub buffer: Buffer,
}

impl ListByDeviceIdQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 1;
}

/// Reply for the ListByDeviceId message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListByDeviceIdReply {
    /// Number of bytes used in the buffer to write the list of devices info (if the call succeeds)
    pub buffer_used_len: usize,
}

/// Parameters for the GetByAddress message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetByAddressQueryParameters {
    /// The address of the device to query for.
    pub address: PciAddress,

    /// Buffer to write the list of devices info into.
    pub buffer: Buffer,
}

impl GetByAddressQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 1;
}

/// Reply for the GetByAddress message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetByAddressReply {
    /// Number of bytes used in the buffer to write the list of devices info (if the call succeeds)
    pub buffer_used_len: usize,
}

/// Parameters for the Open message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenQueryParameters {
    /// The address of the device to open.
    pub address: PciAddress,
}

/// Reply for the Open message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenReply {
    /// Handle to the opened device.
    pub handle: Handle,
}

/// Parameters for the Close message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseQueryParameters {
    /// Handle to the device to close.
    pub handle: Handle,
}

/// Reply for the Close message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseReply {}
