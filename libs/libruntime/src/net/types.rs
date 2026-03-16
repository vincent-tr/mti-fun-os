use core::{fmt, ops::Index};

use crate::kobject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MacAddress([u8; 6]);

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }
}

impl MacAddress {
    pub fn null() -> Self {
        Self([0; 6])
    }

    pub fn is_null(&self) -> bool {
        self.0 == [0; 6]
    }

    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }
}

impl Index<usize> for MacAddress {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

#[derive(Debug)]
pub struct BufferPool {
    /// The number of buffers in the pool.
    pub buffer_count: usize,

    /// The size of each buffer in bytes.
    pub buffer_size: usize,

    /// The memory object backing the buffer pool.
    pub mobj: kobject::MemoryObject,
}

impl BufferPool {
    /// A constant representing an invalid buffer index.
    pub const INVALID_INDEX: u32 = u32::MAX;
}
