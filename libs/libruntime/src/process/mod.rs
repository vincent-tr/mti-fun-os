mod kvblock;
pub mod messages;

pub use kvblock::KVBlock;

use crate::{
    ipc::{self, Handles},
    kobject::{self, KObject},
    process::messages::CreateProcessReply,
};

lazy_static::lazy_static! {
    static ref IPC_CLIENT: ipc::Client = ipc::Client::new(messages::PORT_NAME, messages::VERSION);
}

#[derive(Debug)]
pub struct Process {
    // TODO
}

impl Process {
    pub fn spawn(
        name: &str,
        binary: SharedBuffer,
        env: &[(&str, &str)],
        args: &[(&str, &str)],
    ) -> Self {
        let (name_memobj, name_buffer) = string_to_buffer(name);

        let env = KVBlock::build(env);
        let args = KVBlock::build(args);

        let query_params = messages::CreateProcessQueryParameters {
            name: name_buffer,
            binary_len: binary.size,
        };

        let mut query_handles = Handles::new();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_NAME_MOBJ] =
            name_memobj.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_BINARY_MOBJ] =
            binary.mobj.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_ENV_MOBJ] = env.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_ARGS_MOBJ] =
            args.into_handle();

        let res = IPC_CLIENT.call::<messages::Type, messages::CreateProcessQueryParameters, messages::CreateProcessReply, messages::ProcessServerError>(messages::Type::CreateProcess, query_params, query_handles);

        let (reply, mut reply_handles) = res.expect("failed to create process");

        let process =
            kobject::Process::from_handle(reply_handles.take(CreateProcessReply::HANDLE_PROCESS))
                .expect("failed to get process handle");
        let main_thread = kobject::Thread::from_handle(
            reply_handles.take(CreateProcessReply::HANDLE_MAIN_THREAD),
        )
        .expect("failed to get main thread handle");

        // TODO
        let _ = reply;
        let _ = process;
        let _ = main_thread;

        Self {}
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

/// A buffer that can be shared between processes.
#[derive(Debug)]
pub struct SharedBuffer {
    pub mobj: kobject::MemoryObject,
    pub size: usize, // can be less than mobj.size()
}
