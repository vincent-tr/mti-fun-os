use core::fmt::{Debug, Formatter, Result};

use core::str;

/// Structure of an IPC message
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Message {
    /// User data
    ///
    /// May contain type, transaction id, whatever is relevant.
    ///
    /// If data are bigger than 8x8 bytes, you may use shared memory to pass buffer.
    pub data: [u64; Self::DATA_SIZE],

    /// Handles to transmit from one process to another
    ///
    /// From the sender perspective, the handles are sent: they are consumed, they are not valid after the send operation succeeded.
    ///
    /// From the receiver perspective, the handles are owned by the receiver after the receive operation succeeded.
    ///
    /// Set to invalid if no handle
    ///
    pub handles: [u64; Self::HANDLE_COUNT],
}

impl Message {
    pub const DATA_SIZE: usize = 8;
    pub const HANDLE_COUNT: usize = 4;
}

/// Process information
#[repr(C)]
pub struct PortInfo {
    pub id: u64,
    pub name: [u8; Self::NAME_LEN],
    pub closed: bool,
    pub message_queue_count: usize,
    pub waiting_receiver_count: usize,
}

impl PortInfo {
    pub const NAME_LEN: usize = 128;
}

impl Debug for PortInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("PortInfo")
            .field("id", &self.id)
            .field(
                "name",
                &format_args!("{}", unsafe { str::from_utf8_unchecked(&self.name) }),
            )
            .field("closed", &self.closed)
            .field("message_queue_count", &self.message_queue_count)
            .field("waiting_receiver_count", &self.waiting_receiver_count)
            .finish()
    }
}
