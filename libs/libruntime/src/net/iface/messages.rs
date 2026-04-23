use core::fmt;

use crate::{
    drivers::pci::PciAddress,
    ipc::buffer_messages::Buffer,
    net::types::{IpAddress, IpPrefix},
};

/// Name of the IPC port for the net server.
pub const PORT_NAME: &str = "net";

/// Version of the net messages.
pub const VERSION: u16 = 1;

/// Types of messages used in net management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    CreateInterface = 1,
    DestroyInterface,

    SetRoute,
    RemoveRoute,
    ListRoutes,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}

/// Errors used by net management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum NetServerError {
    InvalidArgument = 1,
    RuntimeError,
    DeviceError,
    BufferTooSmall,
}

impl fmt::Display for NetServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
            Self::DeviceError => write!(f, "DeviceError"),
            Self::BufferTooSmall => write!(f, "BufferTooSmall"),
        }
    }
}

/// Parameters for the CreateInterface message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateInterfaceQueryParameters {
    /// Name of the NIC to create.
    pub name: Buffer,

    /// Port name of the device driver.
    pub driver_port_name: Buffer,

    /// The PCI address of the network device to create.
    pub pci_address: PciAddress,
}

impl CreateInterfaceQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
    pub const HANDLE_DRIVER_PORT_NAME_MOBJ: usize = 2;
}

/// Reply for the CreateInterface message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateInterfaceReply {}

/// Parameters for the DestroyInterface message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DestroyInterfaceQueryParameters {
    /// Name of the NIC to destroy.
    pub name: Buffer,
}

impl DestroyInterfaceQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
}

/// Reply for the DestroyInterface message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DestroyInterfaceReply {}

/// Parameters for the SetRoute message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetRouteQueryParameters {
    /// Network
    pub prefix: IpPrefix,

    /// Optional gateway
    pub gateway: Option<IpAddress>,

    /// Interface used
    pub iface: Buffer,

    /// Metric for this route
    pub metric: usize,
}

impl SetRouteQueryParameters {
    pub const HANDLE_IFACE_MOBJ: usize = 1;
}

/// Reply for the SetRoute message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetRouteReply {}

/// Parameters for the RemoveRoute message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RemoveRouteQueryParameters {
    /// Network
    pub prefix: IpPrefix,

    /// Interface used
    pub iface: Buffer,
}

impl RemoveRouteQueryParameters {
    pub const HANDLE_IFACE_MOBJ: usize = 1;
}

/// Reply for the RemoveRoute message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RemoveRouteReply {}

/// Parameters for the ListRoutes message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListRoutesQueryParameters {
    /// Buffer to write the list of mounts into.
    pub buffer: Buffer,
}

impl ListRoutesQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 1;
}

/// Reply for the ListRoutes message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListRoutesReply {
    /// Number of bytes used in the buffer to write the list of mounts (if the call succeeds)
    pub buffer_used_len: usize,
}
