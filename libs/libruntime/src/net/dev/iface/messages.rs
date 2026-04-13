use bit_field::BitField;
use core::fmt;

use crate::{
    drivers::pci::PciAddress,
    ipc::{self, buffer_messages::Buffer},
    net::types::{BufferPool, MacAddress},
};

/// Version of the net device management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in net device management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    Create = 1,
    Destroy,

    GetLinkStatus,
    SetLinkStatusChangePort,
    GetMacAddress,

    Tx,
    SetTxFreePort,
    AddRxBuffers,
    SetRxPort,
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

    /// Configuration for the net buffer pool to use for this device.
    pub net_buffer_pool: NetBufferPoolConfig,
}

impl CreateQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
    pub const HANDLE_NET_BUFFER_POOL_MOBJ: usize = 2;
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
pub struct SetLinkStatusChangePortReply {}

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

/// Parameters for the Tx message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TxQueryParameters {
    /// Handle to the network device to transmit on.
    pub handle: ipc::Handle,

    /// Buffers containing the data to transmit.
    pub tx_descriptors: [TxBufferDescriptor; Self::DESCRIPTOR_COUNT],
}

impl TxQueryParameters {
    pub const DESCRIPTOR_COUNT: usize = 13;
}

/// Reply for the Tx message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TxReply {
    /// Number of buffers that were accepted for transmission. This may be less than the number of buffers provided.
    pub sent_buffers: usize,
}

/// Parameters for the SetTxFreePort message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetTxFreePortQueryParameters {
    /// Handle to the network device to register for Tx free notifications.
    pub handle: ipc::Handle,

    /// Value to correlate the notification with the registration
    ///
    /// This value will be sent back in the TxFreeNotification
    pub correlation: u64,
}

impl SetTxFreePortQueryParameters {
    /// Note: setting a invalid handle here unsets the port, effectively unregistering from Tx free notifications.
    pub const HANDLE_PORT: usize = 1;
}

/// Reply for the SetTxFreePort message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetTxFreePortReply {}

/// Parameters for the AddRxBuffers message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AddRxBuffersQueryParameters {
    /// Handle to the network device to add Rx buffers to.
    pub handle: ipc::Handle,

    /// Buffers to add for receiving data.
    pub buffers: [u32; Self::BUFFER_COUNT],
}

impl AddRxBuffersQueryParameters {
    pub const BUFFER_COUNT: usize = 26;
}

/// Reply for the AddRxBuffers message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AddRxBuffersReply {
    /// Number of buffers that were successfully added. This may be less than the number of buffers provided.
    pub added_buffers: usize,
}

/// Parameters for the SetRxPort message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetRxPortQueryParameters {
    /// Handle to the network device to register for Rx notifications.
    pub handle: ipc::Handle,

    /// Value to correlate the notification with the registration
    ///
    /// This value will be sent back in the RxArrivedNotification
    pub correlation: u64,
}

impl SetRxPortQueryParameters {
    /// Note: setting a invalid handle here unsets the port, effectively unregistering from Rx notifications.
    pub const HANDLE_PORT: usize = 1;
}

/// Reply for the SetRxPort message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetRxPortReply {}

/// Notification sent by the net device management server when the link status changes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LinkStatusChangedNotification {
    /// Value to correlate the notification with the registration
    pub correlation: u64,

    /// Whether the link is up (true) or down (false).
    pub link_up: bool,
}

/// Notification sent by the net device management server when Tx buffers are freed.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TxFreeNotification {
    /// Value to correlate the notification with the registration
    pub correlation: u64,

    /// Buffers that were freed and can be reused.
    pub buffers: [u32; Self::BUFFER_COUNT],
}

impl TxFreeNotification {
    pub const BUFFER_COUNT: usize = 30;
}

/// Notification sent by the net device management server when new Rx data arrives.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RxArrivedNotification {
    /// Value to correlate the notification with the registration
    pub correlation: u64,

    /// Buffers containing the received data.
    pub rx_descriptors: [RxBufferDescriptor; Self::DESCRIPTOR_COUNT],
}

impl RxArrivedNotification {
    pub const DESCRIPTOR_COUNT: usize = 15;
}

/// Configuration for the net buffer pool to use for a network device.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct NetBufferPoolConfig {
    /// Number of buffers in the pool.
    pub buffer_count: usize,

    /// Size of each buffer in bytes.
    pub buffer_size: usize,
}

/// A compact descriptor for a buffer slice, fitting in exactly 8 bytes.
///
/// Layout (64 bits):
/// - bits 0-31: buffer_index (32 bits, can index 4B buffers)
/// - bits 32-42: offset (11 bits, 0-2047 for 2048-byte buffer)
/// - bits 43-53: length (11 bits, 0-2047)
/// - bit 54: end_of_packet flag
/// - bits 55-63: reserved (9 bits for future use)
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct TxBufferDescriptor(u64);

impl TxBufferDescriptor {
    /// Create a new buffer descriptor.
    pub fn new(buffer_index: usize, offset: usize, length: usize, end_of_packet: bool) -> Self {
        assert!(
            buffer_index <= u32::MAX as usize,
            "buffer_index out of bounds"
        );
        assert!(offset < 2048, "offset must be < 2048");
        assert!(length < 2048, "length must be < 2048");

        let mut value = 0u64;
        value.set_bits(0..32, buffer_index as u64);
        value.set_bits(32..43, offset as u64);
        value.set_bits(43..54, length as u64);
        value.set_bit(54, end_of_packet);

        TxBufferDescriptor(value)
    }

    /// Get the buffer index.
    pub fn buffer_index(&self) -> usize {
        self.0.get_bits(0..32) as usize
    }

    /// Get the offset within the buffer.
    pub fn offset(&self) -> usize {
        self.0.get_bits(32..43) as usize
    }

    /// Get the length of data in the buffer.
    pub fn length(&self) -> usize {
        self.0.get_bits(43..54) as usize
    }

    /// Check if this is the end of packet.
    pub fn end_of_packet(&self) -> bool {
        self.0.get_bit(54)
    }
}

impl fmt::Debug for TxBufferDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxBufferDescriptor")
            .field("buffer_index", &self.buffer_index())
            .field("offset", &self.offset())
            .field("length", &self.length())
            .field("end_of_packet", &self.end_of_packet())
            .finish()
    }
}

impl Default for TxBufferDescriptor {
    fn default() -> Self {
        Self::new(BufferPool::INVALID_INDEX, 0, 0, false)
    }
}

/// A compact descriptor for a receive buffer, fitting in exactly 8 bytes.
///
/// Layout (64 bits):
/// - bits 0-31: buffer_index (32 bits)
/// - bits 32-42: length (11 bits, 0-2047 for 2048-byte buffer)
/// - bit 43: end_of_packet flag
/// - bit 44: error flag (indicates if there was an error receiving into this buffer)
/// - bits 45-63: reserved (19 bits for future use)
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RxBufferDescriptor(u64);

impl RxBufferDescriptor {
    /// Create a new RX buffer descriptor.
    pub fn new(buffer_index: usize, length: usize, end_of_packet: bool, error: bool) -> Self {
        assert!(
            buffer_index <= u32::MAX as usize,
            "buffer_index out of bounds"
        );
        assert!(length <= 2048, "length must be <= 2048");

        let mut value = 0u64;
        value.set_bits(0..32, buffer_index as u64);
        value.set_bits(32..43, length as u64);
        value.set_bit(43, end_of_packet);
        value.set_bit(44, error);

        RxBufferDescriptor(value)
    }

    /// Get the buffer index.
    pub fn buffer_index(&self) -> usize {
        self.0.get_bits(0..32) as usize
    }

    /// Get the length of data in the buffer.
    pub fn length(&self) -> usize {
        self.0.get_bits(32..43) as usize
    }

    /// Check if this is the end of packet.
    pub fn end_of_packet(&self) -> bool {
        self.0.get_bit(43)
    }

    /// Check if there was an error receiving into this buffer.
    pub fn error(&self) -> bool {
        self.0.get_bit(44)
    }
}

impl fmt::Debug for RxBufferDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RxBufferDescriptor")
            .field("buffer_index", &self.buffer_index())
            .field("length", &self.length())
            .field("end_of_packet", &self.end_of_packet())
            .finish()
    }
}

impl Default for RxBufferDescriptor {
    fn default() -> Self {
        Self::new(BufferPool::INVALID_INDEX, 0, false, false)
    }
}
