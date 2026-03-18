use core::{fmt, ops::Index};

use alloc::vec::Vec;
use hashbrown::HashMap;

use crate::{kobject, memory::align_down};

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
    pub const INVALID_INDEX: usize = u32::MAX as usize;
}

#[derive(Debug)]
pub struct PhysBufferPoolAccess {
    /// The size of each buffer in bytes.
    buffer_size: usize,

    /// The physical addresses of the buffers in the pool.
    addresses: Vec<PhysAddr>,

    /// A mapping from physical addresses to buffer indexes.
    indexes: HashMap<PhysAddr, usize>,
}

impl PhysBufferPoolAccess {
    /// Creates a new `PhysBufferPoolAccess` for the given buffer pool.
    pub fn new(buffer_pool: &BufferPool) -> Self {
        let buffer_size = buffer_pool.buffer_size;
        assert!(
            buffer_size.is_power_of_two(),
            "Buffer size must be a power of two"
        );
        assert!(
            buffer_size <= kobject::PAGE_SIZE,
            "Buffer size must be less than or equal to the page size"
        );

        let mut addresses = Vec::with_capacity(buffer_pool.buffer_count);
        let mut indexes = HashMap::with_capacity(buffer_pool.buffer_count);

        for index in 0..buffer_pool.buffer_count {
            let addr = PhysAddr::from(
                buffer_pool
                    .mobj
                    .phys_addr(index * buffer_size)
                    .expect("Could not get physical address"),
            );

            addresses.push(addr);
            indexes.insert(addr, index);
        }

        Self {
            buffer_size,
            addresses,
            indexes,
        }
    }

    /// Returns the physical address of the buffer at the given index, or `None` if the index is invalid.
    pub fn address_of(&self, index: usize) -> Option<PhysAddr> {
        self.addresses.get(index).copied()
    }

    /// Returns the buffer index and offset for the given physical address, or `None` if the address is not part of any buffer in the pool.
    pub fn index_of(&self, addr: PhysAddr) -> Option<(usize, usize)> {
        // Align the address down to the nearest buffer boundary.
        let buffer_addr = PhysAddr::from(align_down(addr.as_u64() as usize, self.buffer_size));
        let offset = (addr.as_u64() - buffer_addr.as_u64()) as usize;
        let index = self.indexes.get(&buffer_addr)?;
        Some((*index, offset))
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct PhysAddr(u64);

impl From<u64> for PhysAddr {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<usize> for PhysAddr {
    fn from(value: usize) -> Self {
        Self(value as u64)
    }
}

impl PhysAddr {
    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}
