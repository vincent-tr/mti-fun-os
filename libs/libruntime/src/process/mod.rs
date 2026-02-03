mod kvblock;
pub mod messages;

pub use kvblock::KVBlock;

use crate::{
    ipc::{self, buffer::Buffer, handle::Handle, CallError, KHandles},
    kobject::KObject,
};

lazy_static::lazy_static! {
    static ref IPC_CLIENT: ipc::Client = ipc::Client::new(messages::PORT_NAME, messages::VERSION);
}

type ProcessServerError = CallError<messages::ProcessServerError>;

#[derive(Debug)]
pub struct Process {
    handle: Handle,
}

impl Process {
    pub fn spawn(
        name: &str,
        binary: Buffer<'_>,
        env: &[(&str, &str)],
        args: &[(&str, &str)],
    ) -> Result<Self, ProcessServerError> {
        let (name_memobj, name_buffer) = Buffer::new_local(name.as_bytes()).into_shared();
        let (binary_mobj, binary_buffer) = binary.into_shared();

        let env = KVBlock::build(env);
        let args = KVBlock::build(args);

        let query_params = messages::CreateProcessQueryParameters {
            name: name_buffer,
            binary: binary_buffer,
        };

        let mut query_handles = KHandles::new();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_NAME_MOBJ] =
            name_memobj.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_BINARY_MOBJ] =
            binary_mobj.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_ENV_MOBJ] = env.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_ARGS_MOBJ] =
            args.into_handle();

        let (reply, _reply_handles) = IPC_CLIENT.call::<messages::Type, messages::CreateProcessQueryParameters, messages::CreateProcessReply, messages::ProcessServerError>(messages::Type::CreateProcess, query_params, query_handles)?;

        Ok(Self {
            handle: reply.handle,
        })
    }
}
