use crate::user::handle::Handle;

/// Structure of an IPC message
#[derive(Debug)]
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
    pub handles: [Handle; Self::HANDLE_COUNT],
}

impl Message {
    pub const DATA_SIZE: usize = 8;
    pub const HANDLE_COUNT: usize = 4;
}
