use core::{fmt, mem, ptr, slice, str};

use alloc::sync::Arc;

use crate::{
    kobject::{Mapping, MemoryObject, PAGE_SIZE, Permissions, Process},
    memory::align_up,
};

/// Version of the KVBlock format.
const VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct KVBlock {
    mobj: MemoryObject,
    mapping: Arc<Mapping<'static>>,
}

impl KVBlock {
    /// Creates a KVBlock from a list of key-value string entries.
    pub fn build(entries: &[(&str, &str)]) -> MemoryObject {
        // Calculate total size
        let mut total_size = mem::size_of::<Header>();
        for (key, value) in entries {
            total_size += mem::size_of::<KVEntry>();
            total_size += key.len();
            total_size += value.len();
            total_size = align_up(total_size, mem::align_of::<KVEntry>());
        }

        let mut builder = KVBlockBuilder::new(total_size);

        builder.set_header(VERSION, entries.len() as u32);
        for (key, value) in entries {
            builder.add_entry(key, value);
        }

        builder.build()
    }

    /// Creates a KVBlock from a memory object.
    ///
    /// Returns an error if the KVBlock version is unsupported.
    pub fn from_memory_object(mobj: MemoryObject) -> Result<Self, KVBlockLoadError> {
        let size = mobj.size().expect("failed to get mobj size");
        let mapping = Arc::new(
            Process::current()
                .map_mem(None, size, Permissions::READ, &mobj, 0)
                .expect("failed to map kvblock memory object"),
        );

        let block = Self { mobj, mapping };

        if block.header().version != VERSION {
            Err(KVBlockLoadError::InvalidVersion)?;
        }

        Ok(block)
    }

    /// Returns the memory object backing this KVBlock.
    pub fn memory_object(&self) -> &MemoryObject {
        &self.mobj
    }

    /// Consumes the KVBlock and returns the underlying memory object.
    pub fn into_memory_object(self) -> MemoryObject {
        self.mobj
    }

    /// Safety: The caller must ensure that the offset points to a valid T within the mapping.
    unsafe fn read<T: Sized>(&self, offset: usize) -> &T {
        assert!(offset + mem::size_of::<T>() <= self.mapping.len());
        assert!((self.mapping.address() + offset) % mem::align_of::<T>() == 0);

        &*((self.mapping.address() + offset) as *const T)
    }

    fn header(&self) -> &Header {
        unsafe { self.read(0) }
    }

    fn entry(&self, offset: usize) -> &KVEntry {
        let entry = unsafe { self.read::<KVEntry>(offset) };
        assert!(offset + entry.total_size() <= self.mapping.len());

        entry
    }

    /// Returns the number of key-value entries in the KVBlock.
    pub fn len(&self) -> usize {
        self.header().entry_count as usize
    }

    /// Returns an iterator over the key-value entries in the KVBlock.
    pub fn iter(&self) -> KVBlockIterator {
        KVBlockIterator {
            owner: self,
            current_index: 0,
            current_offset: align_up(mem::size_of::<Header>(), mem::align_of::<KVEntry>()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KVBlockLoadError {
    InvalidVersion,
}

impl fmt::Display for KVBlockLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersion => write!(f, "InvalidVersion"),
        }
    }
}

impl core::error::Error for KVBlockLoadError {}

/// Iterator over the key-value entries in a KVBlock.
#[derive(Debug)]
pub struct KVBlockIterator<'a> {
    owner: &'a KVBlock,
    current_index: u32,
    current_offset: usize,
}

impl<'a> Iterator for KVBlockIterator<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.owner.header().entry_count {
            return None;
        }

        let entry = self.owner.entry(self.current_offset);
        let key = unsafe { entry.key() };
        let value = unsafe { entry.value() };

        self.current_offset += align_up(entry.total_size(), mem::align_of::<KVEntry>());
        self.current_index += 1;

        Some((key, value))
    }
}

/// Builder for creating a KVBlock.
#[derive(Debug)]
struct KVBlockBuilder {
    mobj: MemoryObject,
    mapping: Mapping<'static>,
    current_entry_offset: usize,
}

impl KVBlockBuilder {
    pub fn new(total_size: usize) -> Self {
        let mobj_size = align_up(total_size, PAGE_SIZE);
        let mobj = MemoryObject::create(mobj_size).expect("failed to create mobj");
        let mapping = Process::current()
            .map_mem(
                None,
                mobj_size,
                Permissions::READ | Permissions::WRITE,
                &mobj,
                0,
            )
            .expect("failed to map kvblock memory object");

        let current_entry_offset = align_up(mem::size_of::<Header>(), mem::align_of::<KVEntry>());

        Self {
            mobj,
            mapping,
            current_entry_offset,
        }
    }

    pub fn set_header(&mut self, version: u32, entry_count: u32) {
        let header = self.header_mut();
        header.version = version;
        header.entry_count = entry_count;
    }

    pub fn add_entry(&mut self, key: &str, value: &str) {
        let offset = self.current_entry_offset;
        let mut data_addr =
            self.mapping.address() + self.current_entry_offset + mem::size_of::<KVEntry>();

        let entry = self.entry_mut(offset);
        entry.key_len = key.len() as u32;
        entry.value_len = value.len() as u32;

        unsafe {
            ptr::copy_nonoverlapping(key.as_ptr(), data_addr as *mut u8, key.len());
            data_addr += key.len();
            ptr::copy_nonoverlapping(value.as_ptr(), data_addr as *mut u8, value.len());
        }

        self.current_entry_offset += align_up(entry.total_size(), mem::align_of::<KVEntry>());
    }

    pub fn build(self) -> MemoryObject {
        self.mobj
    }

    fn header_mut(&mut self) -> &mut Header {
        self.read_mut::<Header>(0)
    }

    fn entry_mut(&mut self, offset: usize) -> &mut KVEntry {
        self.read_mut::<KVEntry>(offset)
    }

    fn read_mut<T: Sized>(&mut self, offset: usize) -> &mut T {
        assert!(offset + mem::size_of::<T>() <= self.mapping.len());
        assert!((self.mapping.address() + offset) % mem::align_of::<T>() == 0);

        unsafe { &mut *((self.mapping.address() + offset) as *mut T) }
    }
}

#[derive(Debug)]
#[repr(C)]
struct Header {
    pub version: u32,
    pub entry_count: u32,
}

#[derive(Debug)]
#[repr(C)]
struct KVEntry {
    pub key_len: u32,
    pub value_len: u32,
}

impl KVEntry {
    pub fn total_size(&self) -> usize {
        mem::size_of::<KVEntry>() + self.key_len as usize + self.value_len as usize
    }

    /// Safety: The caller must ensure that the KVEntry is valid and followed by valid string data.
    pub unsafe fn key(&self) -> &str {
        let start_addr = (self as *const KVEntry as usize) + mem::size_of::<KVEntry>();
        let buffer = slice::from_raw_parts(start_addr as *const u8, self.key_len as usize);
        str::from_utf8_unchecked(buffer)
    }

    /// Safety: The caller must ensure that the KVEntry is valid and followed by valid string data.
    pub unsafe fn value(&self) -> &str {
        let start_addr =
            (self as *const KVEntry as usize) + mem::size_of::<KVEntry>() + self.key_len as usize;
        let buffer = slice::from_raw_parts(start_addr as *const u8, self.value_len as usize);
        str::from_utf8_unchecked(buffer)
    }
}
