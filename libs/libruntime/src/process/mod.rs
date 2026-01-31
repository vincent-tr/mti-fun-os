mod kvblock;
pub mod messages;

pub use kvblock::KVBlock;

use crate::{
    ipc::{self, buffer::Buffer, Handles},
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
        binary: Buffer<'_>,
        env: &[(&str, &str)],
        args: &[(&str, &str)],
    ) -> Self {
        let (name_memobj, name_buffer) = Buffer::from(name.as_bytes()).into_shared();
        let (binary_mobj, binary_buffer) = binary.into_shared();

        let env = KVBlock::build(env);
        let args = KVBlock::build(args);

        let query_params = messages::CreateProcessQueryParameters {
            name: name_buffer,
            binary: binary_buffer,
        };

        let mut query_handles = Handles::new();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_NAME_MOBJ] =
            name_memobj.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_BINARY_MOBJ] =
            binary_mobj.into_handle();
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
