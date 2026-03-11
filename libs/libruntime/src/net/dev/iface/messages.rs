use core::fmt;

use crate::{
    drivers::pci::PciAddress,
    ipc::{self, buffer_messages::Buffer},
    net::types::MacAddress,
};

/// Version of the net device management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in net device management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    Create,
    Destroy,

    GetLinkStatus,
    SetLinkStatusChangePort,
    GetMacAddress,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}

/// Errors used by net device management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum NetDeviceError {
    InvalidArgument = 1,
    RuntimeError,
    DeviceError,
}

impl fmt::Display for NetDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
            Self::DeviceError => write!(f, "DeviceError"),
        }
    }
}

/// Parameters for the Create message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateQueryParameters {
    /// Name of the NIC to create. This is used for logging and debugging purposes.
    pub name: Buffer,

    /// The PCI address of the network device to create.
    pub pci_address: PciAddress,
}

impl CreateQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
}

/// Reply for the Create message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateReply {
    /// Handle to the created network device.
    pub handle: ipc::Handle,
}

/// Parameters for the Destroy message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DestroyQueryParameters {
    /// Handle to the network device to destroy.
    pub handle: ipc::Handle,
}

/// Reply for the Destroy message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DestroyReply {}

/// Parameters for the GetLinkStatus message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetLinkStatusQueryParameters {
    /// Handle to the network device to query.
    pub handle: ipc::Handle,
}

/// Reply for the GetLinkStatus message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetLinkStatusReply {
    /// Whether the link is up (true) or down (false).
    pub link_up: bool,
}

/// Parameters for the SetLinkStatusChangePort message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetLinkStatusChangePortQueryParameters {
    /// Handle to the network device to register for link status change notifications.
    pub handle: ipc::Handle,

    /// Value to correlate the notification with the registration
    ///
    /// This value will be sent back in the LinkStatusChangedNotification
    pub correlation: u64,
}

impl SetLinkStatusChangePortQueryParameters {
    /// Note: setting a invalid handle here unsets the port, effectively unregistering from link status change notifications.
    pub const HANDLE_PORT: usize = 1;
}

/// Reply for the SetLinkStatusChangePort message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetLinkStatusChangePortReply {
    /// Handle to the registration, used for later unregistration.
    pub registration_handle: ipc::Handle,
}

/// Parameters for the GetMacAddress message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetMacAddressQueryParameters {
    /// Handle to the network device to query.
    pub handle: ipc::Handle,
}

/// Reply for the GetMacAddress message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetMacAddressReply {
    /// The MAC address of the network device.
    pub mac_address: MacAddress,
}

/// Notification sent by the net device management server when the link status changes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LinkStatusChangedNotification {
    /// Value to correlate the notification with the registration
    pub correlation: u64,

    /// Whether the link is up (true) or down (false).
    pub link_up: bool,
}
