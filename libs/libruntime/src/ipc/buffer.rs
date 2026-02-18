use core::slice;

use crate::{
    kobject::{self, KObject, Permissions},
    memory,
};

pub mod messages {
    /// A buffer descriptor used in IPC messages.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct Buffer {
        // will take a handle slot for MemoryObject
        pub offset: usize,
        pub size: usize,
    }
}

/// A buffer that can be either local or shared via a memory object.
#[derive(Debug)]
pub enum Buffer<'a> {
    Local(&'a [u8]),
    Shared((kobject::MemoryObject, messages::Buffer)),
}

/// Create a storage for empty buffers, to avoid having null pointers in mapping, which is rejected by the kernel.
const EMPTY_BUFFER_STORAGE: [u8; 1] = [0];

impl<'a> Buffer<'a> {
    pub fn new_local(data: &'a [u8]) -> Self {
        Self::Local(data)
    }

    pub fn new_shared(mobj: kobject::MemoryObject, offset: usize, size: usize) -> Self {
        Self::Shared((mobj, messages::Buffer { offset, size }))
    }

    pub fn into_shared(self) -> (kobject::MemoryObject, messages::Buffer) {
        match self {
            Self::Local(data) => Self::to_buffer(data),
            Self::Shared((mobj, buffer)) => (mobj, buffer),
        }
    }

    fn to_buffer(mut value: &[u8]) -> (kobject::MemoryObject, messages::Buffer) {
        if value.is_empty() {
            // get correct pointer even for empty buffers, to permit mobj resolution
            value = &EMPTY_BUFFER_STORAGE.as_slice()[0..0];
        }

        let process = kobject::Process::current();
        // Let's consider that the whole buffer is laid in a single memory object for now
        let info = process
            .map_info(value.as_ptr() as usize)
            .expect("failed to get address info");
        assert!(info.perms.contains(kobject::Permissions::READ));
        let mobj = info.mobj.expect("buffer has no backing memory object");

        let buffer = messages::Buffer {
            offset: info.offset,
            size: value.len(),
        };

        (mobj, buffer)
    }
}

/// Reader for a buffer shared via a memory object.
#[derive(Debug)]
pub struct BufferView {
    mapping: Option<kobject::Mapping<'static>>, // None for empty buffers
    address: usize,
    size: usize,
}

#[derive(Debug)]
pub enum BufferViewAccess {
    ReadOnly,
    ReadWrite,
}

impl BufferView {
    /// Create a new BufferView from a handle to a memory object and a buffer descriptor.
    pub fn new(
        handle: kobject::Handle,
        buffer: &messages::Buffer,
        access: BufferViewAccess,
    ) -> Result<Self, kobject::Error> {
        if buffer.size == 0 {
            // For empty buffers, we can skip mapping and just return a view with an empty range.
            return Ok(Self {
                mapping: None,
                address: 0,
                size: 0,
            });
        }

        let mem_obj = kobject::MemoryObject::from_handle(handle)?;

        let process = kobject::Process::current();

        // align mapping to page boundaries
        let buffer_begin = buffer.offset;
        let buffer_end = buffer.offset + buffer.size;
        let mapping_begin = memory::align_down(buffer_begin, kobject::PAGE_SIZE);
        let mapping_end = memory::align_up(buffer_end, kobject::PAGE_SIZE);
        let mapping_size = mapping_end - mapping_begin;

        let perms = match access {
            BufferViewAccess::ReadOnly => Permissions::READ,
            BufferViewAccess::ReadWrite => Permissions::READ | Permissions::WRITE,
        };

        let mapping = process.map_mem(None, mapping_size, perms, &mem_obj, mapping_begin)?;

        let range_begin = buffer_begin - mapping_begin;

        Ok(Self {
            address: mapping.address() + range_begin,
            mapping: Some(mapping),
            size: buffer.size,
        })
    }

    /// Get a slice to the buffer's data.
    pub fn buffer(&self) -> &[u8] {
        unsafe { slice::from_raw_parts((self.address) as *const _, self.size) }
    }

    /// Get a mutable slice to the buffer's data.
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        if let Some(mapping) = &self.mapping {
            assert!(mapping.permissions().contains(Permissions::WRITE));
        }

        unsafe { slice::from_raw_parts_mut((self.address) as *mut _, self.size) }
    }

    /// Get the buffer's data as a string slice.
    ///
    /// Safety: The buffer must contain valid UTF-8 data.
    pub unsafe fn str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(self.buffer()) }
    }
}
