use core::fmt::{Debug, Formatter, Result};

use core::str;

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
