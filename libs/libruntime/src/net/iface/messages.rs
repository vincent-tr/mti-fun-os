use core::fmt;

use crate::{
    drivers::pci::PciAddress,
    ipc::buffer_messages::Buffer,
    net::types::{IpAddress, IpPrefix, MacAddress},
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
    SetInterfaceConfig,
    GetInterfaceConfig,
    GetInterfaceInfo,

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
    NotSupported,
    RuntimeError,
    DeviceError,
    BufferTooSmall,
}

impl fmt::Display for NetServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::NotSupported => write!(f, "NotSupported"),
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

/// Parameters for the SetInterfaceConfig message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetInterfaceConfigQueryParameters {
    /// Name of the NIC to configure.
    pub name: Buffer,

    /// Configuration of the interface
    pub config: InterfaceConfig,
}

impl SetInterfaceConfigQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
}

/// Reply for the SetInterfaceConfig message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetInterfaceConfigReply {}

/// Parameters for the GetInterfaceConfig message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetInterfaceConfigQueryParameters {
    /// Name of the NIC to configure.
    pub name: Buffer,
}

impl GetInterfaceConfigQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
}

/// Reply for the GetInterfaceConfig message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetInterfaceConfigReply {
    /// Configuration of the interface
    pub config: InterfaceConfig,
}

/// Parameters for the GetInterfaceInfo message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetInterfaceInfoQueryParameters {
    /// Name of the NIC to configure.
    pub name: Buffer,
}

impl GetInterfaceInfoQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
}

/// Reply for the GetInterfaceInfo message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetInterfaceInfoReply {
    /// Info for the interface
    pub info: InterfaceInfo,
}

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

/// Interface configuration
#[derive(Debug, Clone, Copy)]
pub enum InterfaceConfig {
    /// Dynamic configuration (DHCP)
    Dynamic,

    /// Static configuration
    Static(StaticInterfaceConfig),
}

/// Static interface configuration
#[derive(Debug, Clone, Copy)]
pub struct StaticInterfaceConfig {
    /// IP address of the interface
    pub ip_address: IpAddress,

    /// Subnet mask of the interface
    pub subnet_mask: IpAddress,
}

/// Interface info
#[derive(Debug, Clone, Copy)]
pub struct InterfaceInfo {
    /// Mac address of the interface
    pub mac_address: MacAddress,

    /// IP address and subnet mask of the interface
    pub ip_address: Option<(IpAddress, IpAddress)>,
}
