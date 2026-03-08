use core::fmt;

use crate::{
    drivers::pci::types::PciAddress,
    ipc::{Handle, buffer_messages::Buffer},
};

use super::PciDeviceInfo;

/// Name of the IPC port for the PCI server.
pub const PORT_NAME: &str = "pci-server";

/// Version of the PCI interface management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in PCI interface management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    // Discovery messages
    List,
    GetByAddress,

    // Handles management messages
    Open,
    Close,
    GetHeader,
    Enable,
    ListCapabilities,
    ReadCapability,
    WriteCapability,
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
    DeviceNotFound,
    DeviceInUse,
}

impl fmt::Display for PciServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
            Self::DeviceNotFound => write!(f, "DeviceNotFound"),
            Self::DeviceInUse => write!(f, "DeviceInUse"),
        }
    }
}

/// Parameters for the ListByClass message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListQueryParameters {
    /// The vendor ID to query for (16 bits).
    pub vendor_id: Option<u16>,

    /// The device ID to query for (16 bits).
    pub device_id: Option<u16>,

    /// The class code to query for (8 bits).
    pub class: Option<u8>,

    /// The subclass code to query for (8 bits).
    pub subclass: Option<u8>,

    /// Buffer to write the list of devices info into.
    pub buffer: Buffer,
}

impl ListQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 1;
}

/// Reply for the List message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListReply {
    /// Number of bytes used in the buffer to write the list of devices info (if the call succeeds)
    pub buffer_used_len: usize,
}

/// Parameters for the GetByAddress message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetByAddressQueryParameters {
    /// The address of the device to query for.
    pub address: PciAddress,
}

/// Reply for the GetByAddress message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetByAddressReply {
    pub device_info: PciDeviceInfo,
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

/// Parameters for the GetHeader message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetHeaderQueryParameters {
    /// Handle to the device to get the header from.
    pub handle: Handle,

    /// Buffer to write the header into.
    pub header_buffer: Buffer,
}

impl GetHeaderQueryParameters {
    pub const HANDLE_HEADER_BUFFER_MOBJ: usize = 1;
}

/// Reply for the GetHeader message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetHeaderReply {}

/// Parameters for the Enable message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EnableQueryParameters {
    /// Handle to the device to enable.
    pub handle: Handle,

    /// Whether to enable memory access for the device.
    pub memory: bool,

    /// Whether to enable I/O access for the device.
    pub io: bool,

    /// Whether to enable bus mastering for the device.
    pub bus_master: bool,
}

/// Reply for the Enable message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EnableReply {}

/// Parameters for the ListCapabilities message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListCapabilitiesQueryParameters {
    /// Handle to the device to list capabilities for.
    pub handle: Handle,

    /// Buffer to write the capabilities list into.
    pub buffer: Buffer,
}

impl ListCapabilitiesQueryParameters {
    pub const HANDLE_CAPABILITIES_BUFFER_MOBJ: usize = 1;
}

/// Reply for the ListCapabilities message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListCapabilitiesReply {
    /// Number of bytes used in the buffer to write the capabilities list (if the call succeeds)
    pub buffer_used_len: usize,
}

/// Parameters for the ReadCapability message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadCapabilityQueryParameters {
    /// Handle to the device to read the capability from.
    pub handle: Handle,

    /// Index of the capability to read.
    pub capability_index: usize,

    /// Offset within the capability data to read from.
    pub offset: usize,

    /// Size of the capability data to read.
    pub size: usize,

    /// Buffer to write the capability data into.
    pub buffer: Buffer,
}

impl ReadCapabilityQueryParameters {
    pub const HANDLE_CAPABILITY_BUFFER_MOBJ: usize = 1;
}

/// Reply for the ReadCapability message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadCapabilityReply {}

/// Parameters for the WriteCapability message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteCapabilityQueryParameters {
    /// Handle to the device to write the capability to.
    pub handle: Handle,

    /// Index of the capability to write.
    pub capability_index: usize,

    /// Offset within the capability data to write to.
    pub offset: usize,

    /// Size of the capability data to write.
    pub size: usize,

    /// Buffer containing the capability data to write.
    pub buffer: Buffer,
}

impl WriteCapabilityQueryParameters {
    pub const HANDLE_CAPABILITY_BUFFER_MOBJ: usize = 1;
}

/// Reply for the WriteCapability message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteCapabilityReply {}
