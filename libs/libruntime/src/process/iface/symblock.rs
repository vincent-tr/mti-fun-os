use core::{fmt, mem, ptr, slice, str};

use alloc::{collections::BTreeMap, string::String, sync::Arc};

use crate::{
    kobject::{Mapping, MemoryObject, PAGE_SIZE, Permissions, Process},
    memory::align_up,
};

/// Version of the SymBlock format.
const VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct SymBlock {
    mobj: MemoryObject,
    mapping: Arc<Mapping<'static>>,
}

impl SymBlock {
    /// Creates a SymBlock from a BTreeMap of address-to-name entries.
    /// The BTreeMap ensures entries are sorted by address.
    pub fn build(entries: &BTreeMap<usize, String>) -> SymBlock {
        // Calculate total size: header + all entries + all strings
        let header_size = mem::size_of::<Header>();
        let entries_size = entries.len() * mem::size_of::<SymEntry>();
        let strings_size: usize = entries.values().map(|s| s.len()).sum();
        let total_size = header_size + entries_size + strings_size;

        let mut builder = SymBlockBuilder::new(total_size);

        // Set header with strings offset
        let strings_offset = header_size + entries_size;
        builder.set_header(VERSION, entries.len() as u32, strings_offset as u32);

        // Add all entries and strings
        builder.add_entries(entries);

        builder.build()
    }

    /// Creates a SymBlock from a memory object.
    ///
    /// Returns an error if the SymBlock version is unsupported.
    pub fn from_memory_object(mobj: MemoryObject) -> Result<Self, SymBlockLoadError> {
        let size = mobj.size().expect("failed to get mobj size");
        let mapping = Arc::new(
            Process::current()
                .map_mem(None, size, Permissions::READ, &mobj, 0)
                .expect("failed to map symblock memory object"),
        );

        let block = Self { mobj, mapping };

        if block.header().version != VERSION {
            Err(SymBlockLoadError::InvalidVersion)?;
        }

        Ok(block)
    }

    /// Returns the memory object backing this SymBlock.
    pub fn memory_object(&self) -> &MemoryObject {
        &self.mobj
    }

    /// Consumes the SymBlock and returns the underlying memory object.
    pub fn into_memory_object(self) -> MemoryObject {
        self.mobj
    }

    /// Safety: The caller must ensure that the offset points to a valid T within the mapping.
    unsafe fn read<T: Sized>(&self, offset: usize) -> &T {
        assert!(offset + mem::size_of::<T>() <= self.mapping.len());
        assert!((self.mapping.address() + offset) % mem::align_of::<T>() == 0);

        unsafe { &*((self.mapping.address() + offset) as *const T) }
    }

    fn header(&self) -> &Header {
        unsafe { self.read(0) }
    }

    fn entry(&self, offset: usize) -> &SymEntry {
        unsafe { self.read::<SymEntry>(offset) }
    }

    /// Returns the number of symbol entries in the SymBlock.
    pub fn len(&self) -> usize {
        self.header().entry_count as usize
    }

    /// Returns an iterator over the symbol entries in the SymBlock.
    pub fn iter(&self) -> SymBlockIterator {
        SymBlockIterator {
            owner: self,
            current_index: 0,
            current_offset: mem::size_of::<Header>(),
        }
    }

    /// Looks up a symbol by address using binary search.
    ///
    /// Finds the entry where `address >= entry.address` and `address < next_entry.address`.
    /// Returns (symbol_address, symbol_name) if found, None otherwise.
    pub fn lookup(&self, address: u64) -> Option<(u64, &str)> {
        let entry_count = self.header().entry_count as usize;
        if entry_count == 0 {
            return None;
        }

        let entries_offset = mem::size_of::<Header>();

        // Binary search for the largest address <= target
        let mut left = 0;
        let mut right = entry_count;
        let mut result_idx = None;

        while left < right {
            let mid = left + (right - left) / 2;
            let entry_offset = entries_offset + mid * mem::size_of::<SymEntry>();
            let entry = self.entry(entry_offset);

            if entry.address <= address {
                result_idx = Some(mid);
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        result_idx.map(|idx| {
            let entry_offset = entries_offset + idx * mem::size_of::<SymEntry>();
            let entry = self.entry(entry_offset);
            let name = unsafe { self.get_string(entry.string_offset, entry.string_len) };
            (entry.address, name)
        })
    }

    /// Safety: The caller must ensure that string_offset and string_len are valid.
    unsafe fn get_string(&self, string_offset: u32, string_len: u32) -> &str {
        let start_addr = self.mapping.address() + string_offset as usize;
        let buffer = unsafe { slice::from_raw_parts(start_addr as *const u8, string_len as usize) };
        unsafe { str::from_utf8_unchecked(buffer) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymBlockLoadError {
    InvalidVersion,
}

impl fmt::Display for SymBlockLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersion => write!(f, "InvalidVersion"),
        }
    }
}

impl core::error::Error for SymBlockLoadError {}

/// Iterator over the symbol entries in a SymBlock.
#[derive(Debug)]
pub struct SymBlockIterator<'a> {
    owner: &'a SymBlock,
    current_index: u32,
    current_offset: usize,
}

impl<'a> Iterator for SymBlockIterator<'a> {
    type Item = (u64, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.owner.header().entry_count {
            return None;
        }

        let entry = self.owner.entry(self.current_offset);
        let address = entry.address;
        let name = unsafe { self.owner.get_string(entry.string_offset, entry.string_len) };

        self.current_offset += mem::size_of::<SymEntry>();
        self.current_index += 1;

        Some((address, name))
    }
}

/// Builder for creating a SymBlock.
#[derive(Debug)]
struct SymBlockBuilder {
    mobj: MemoryObject,
    mapping: Mapping<'static>,
    entries_offset: usize,
    strings_offset: usize,
}

impl SymBlockBuilder {
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
            .expect("failed to map symblock memory object");

        let entries_offset = mem::size_of::<Header>();

        Self {
            mobj,
            mapping,
            entries_offset,
            strings_offset: 0, // Will be set in set_header
        }
    }

    pub fn set_header(&mut self, version: u32, entry_count: u32, strings_offset: u32) {
        let header = self.header_mut();
        header.version = version;
        header.entry_count = entry_count;
        header.strings_offset = strings_offset;
        self.strings_offset = strings_offset as usize;
    }

    pub fn add_entries(&mut self, entries: &BTreeMap<usize, String>) {
        let mut current_entry_idx = 0;
        let mut current_string_offset = self.strings_offset;

        for (address, name) in entries {
            let entry_offset = self.entries_offset + current_entry_idx * mem::size_of::<SymEntry>();
            let entry = self.entry_mut(entry_offset);
            entry.address = *address as u64;
            entry.string_offset = current_string_offset as u32;
            entry.string_len = name.len() as u32;

            // Copy string data
            let string_addr = self.mapping.address() + current_string_offset;
            unsafe {
                ptr::copy_nonoverlapping(name.as_ptr(), string_addr as *mut u8, name.len());
            }

            current_entry_idx += 1;
            current_string_offset += name.len();
        }
    }

    pub fn build(self) -> SymBlock {
        SymBlock {
            mobj: self.mobj,
            mapping: Arc::new(self.mapping),
        }
    }

    fn header_mut(&mut self) -> &mut Header {
        self.read_mut::<Header>(0)
    }

    fn entry_mut(&mut self, offset: usize) -> &mut SymEntry {
        self.read_mut::<SymEntry>(offset)
    }

    fn read_mut<T: Sized>(&mut self, offset: usize) -> &mut T {
        assert!(offset + mem::size_of::<T>() <= self.mapping.len());
        assert!((self.mapping.address() + offset) % mem::align_of::<T>() == 0);

        unsafe { &mut *((self.mapping.address() + offset) as *mut T) }
    }
}

#[derive(Debug)]
#[repr(C, align(8))]
struct Header {
    pub version: u32,
    pub entry_count: u32,
    pub strings_offset: u32,
    // Padding to ensure 8-byte alignment for following SymEntry array
    _padding: u32,
}

#[derive(Debug)]
#[repr(C, align(8))]
struct SymEntry {
    pub address: u64,
    pub string_offset: u32,
    pub string_len: u32,
}
