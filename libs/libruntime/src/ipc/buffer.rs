use core::ops::Range;

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

impl<'a> From<&'a [u8]> for Buffer<'a> {
    fn from(value: &'a [u8]) -> Self {
        Buffer::Local(value)
    }
}

impl Buffer<'_> {
    pub fn new_shared(mobj: kobject::MemoryObject, offset: usize, size: usize) -> Self {
        Self::Shared((mobj, messages::Buffer { offset, size }))
    }

    pub fn into_shared(self) -> (kobject::MemoryObject, messages::Buffer) {
        match self {
            Self::Local(data) => Self::to_buffer(data),
            Self::Shared((mobj, buffer)) => (mobj, buffer),
        }
    }

    fn to_buffer(value: &[u8]) -> (kobject::MemoryObject, messages::Buffer) {
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
pub struct BufferReader {
    mapping: kobject::Mapping<'static>,
    range: Range<usize>,
}

impl BufferReader {
    /// Create a new BufferReader from a handle to a memory object and a buffer descriptor.
    pub fn new(handle: kobject::Handle, buffer: &messages::Buffer) -> Result<Self, kobject::Error> {
        let mem_obj = kobject::MemoryObject::from_handle(handle)?;

        let process = kobject::Process::current();

        // align mapping to page boundaries
        let buffer_begin = buffer.offset;
        let buffer_end = buffer.offset + buffer.size;
        let mapping_begin = memory::align_down(buffer_begin, kobject::PAGE_SIZE);
        let mapping_end = memory::align_up(buffer_end, kobject::PAGE_SIZE);
        let mapping_size = mapping_end - mapping_begin;

        let mapping = process.map_mem(
            None,
            mapping_size,
            Permissions::READ,
            &mem_obj,
            mapping_begin,
        )?;

        let range_begin = buffer_begin - mapping_begin;
        let range_end = range_begin + buffer.size;

        Ok(Self {
            mapping,
            range: range_begin..range_end,
        })
    }

    /// Get a slice to the buffer's data.
    pub fn buffer(&self) -> &[u8] {
        unsafe { self.mapping.as_buffer() }.expect("failed to get buffer")[self.range.clone()]
            .as_ref()
    }
}
