use core::fmt::{Debug, Formatter, Result};

use core::str;

use crate::Permissions;

/// Process information
#[repr(C)]
pub struct ProcessInfo {
    pub pid: u64,
    pub name: [u8; Self::NAME_LEN],
    pub thread_count: usize,
    pub mapping_count: usize,
    pub handle_count: usize,
    pub terminated: bool,
}

impl ProcessInfo {
    pub const NAME_LEN: usize = 128;
}

impl Debug for ProcessInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ProcessInfo")
            .field("pid", &self.pid)
            .field(
                "name",
                &format_args!("{}", unsafe { str::from_utf8_unchecked(&self.name) }),
            )
            .field("thread_count", &self.thread_count)
            .field("mapping_count", &self.mapping_count)
            .field("handle_count", &self.handle_count)
            .field("terminated", &self.terminated)
            .finish()
    }
}

/// Address info in a process
#[repr(C)]
pub struct AddressInfo {
    pub perms: Permissions,
    pub mobj: u64, // Handle value of the memory object
    pub offset: usize,
}
