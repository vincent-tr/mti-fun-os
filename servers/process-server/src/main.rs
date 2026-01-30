#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;
extern crate libruntime;

use core::{mem, ops::Range};

use log::{error, info};

use libruntime::{
    ipc,
    kobject::{self, KObject, Permissions},
    memory,
    process::messages,
};

#[no_mangle]
pub fn main() {
    let server = ipc::ServerBuilder::new(messages::PORT_NAME, messages::VERSION)
        .with_handler(messages::Type::CreateProcess, create_process)
        .build()
        .expect("failed to build process-server IPC server");

    info!("process-server started");

    server.run();
}

fn create_process(
    query: messages::CreateProcessQueryParameters,
    mut query_handles: ipc::Handles,
) -> Result<(messages::CreateProcessReply, ipc::Handles), messages::ProcessServerError> {
    let mut name_handle = kobject::Handle::invalid();
    mem::swap(&mut name_handle, &mut query_handles[1]);

    let name_reader =
        BufferReader::new(name_handle, &query.name).expect("failed to create buffer reader");
    let str = unsafe { str::from_utf8_unchecked(name_reader.buffer()) };

    info!("Creating process {}", str);

    Err(messages::ProcessServerError::InvalidArgument)
}

struct BufferReader {
    mapping: kobject::Mapping<'static>,
    range: Range<usize>,
}

impl BufferReader {
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

    pub fn buffer(&self) -> &[u8] {
        unsafe { self.mapping.as_buffer() }.expect("failed to get buffer")[self.range.clone()]
            .as_ref()
    }
}
