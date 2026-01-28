pub mod messages;

use core::mem;

use libsyscalls::Handle;

use crate::{
    ipc,
    kobject::{self, KObject},
};

lazy_static::lazy_static! {
    static ref IPC_CLIENT: ipc::Client = ipc::Client::new(messages::PORT_NAME, messages::VERSION);
}

#[derive(Debug)]
pub struct Process {
    kobj: kobject::Process,
}

impl Process {
    pub fn spawn(name: &str) -> Self {
        let (name_memobj, name_buffer) = string_to_buffer(name);

        // TODO: binary, env, arg

        let query_params = messages::CreateProcessQueryParameters { name: name_buffer };

        let query_handles = [
            kobject::Handle::invalid(), // reserved for reply
            name_memobj.into_handle(),
            kobject::Handle::invalid(),
            kobject::Handle::invalid(),
        ];

        let res = IPC_CLIENT.call::<messages::Type, messages::CreateProcessQueryParameters, messages::CreateProcessReply, messages::ProcessServerError>(messages::Type::CreateProcess, query_params, query_handles);

        let (reply, mut reply_handles) = res.expect("failed to create process");

        // expected handles: [create process handle, main thread handle]

        let mut process_handle = Handle::invalid();
        mem::swap(&mut process_handle, &mut reply_handles[0]);

        let process =
            kobject::Process::from_handle(process_handle).expect("failed to get process handle");

        Self { kobj: process }
    }
}

fn string_to_buffer(value: &str) -> (kobject::MemoryObject, messages::Buffer) {
    let process = kobject::Process::current();
    // Let's consider that the whole string is laid in a single memory object for now
    let info = process
        .map_info(value.as_ptr() as usize)
        .expect("failed to get address info");
    assert!(info.perms.contains(kobject::Permissions::READ));
    let mobj = info.mobj.expect("string has no backing memory object");

    let buffer = messages::Buffer {
        offset: info.offset,
        size: value.len(),
    };

    (mobj, buffer)
}
