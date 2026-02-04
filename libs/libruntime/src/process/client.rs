use super::{messages, KVBlock};
use crate::{ipc, kobject::KObject};

/// Low level process client implementation.
#[derive(Debug)]
pub struct Client {
    ipc_client: ipc::Client,
}

impl Client {
    /// Creates a new process client.
    pub fn new() -> Self {
        Self {
            ipc_client: ipc::Client::new(messages::PORT_NAME, messages::VERSION),
        }
    }

    /// Creates a new process.
    pub fn create_process(
        &self,
        name: &str,
        binary: ipc::Buffer<'_>,
        env: &[(&str, &str)],
        args: &[(&str, &str)],
    ) -> Result<ipc::Handle, ipc::CallError<messages::ProcessServerError>> {
        let env = KVBlock::build(env);
        let args = KVBlock::build(args);

        let (name_memobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();
        let (binary_mobj, binary_buffer) = binary.into_shared();

        let query_params = messages::CreateProcessQueryParameters {
            name: name_buffer,
            binary: binary_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_NAME_MOBJ] =
            name_memobj.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_BINARY_MOBJ] =
            binary_mobj.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_ENV_MOBJ] = env.into_handle();
        query_handles[messages::CreateProcessQueryParameters::HANDLE_ARGS_MOBJ] =
            args.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::CreateProcessQueryParameters, messages::CreateProcessReply, messages::ProcessServerError>(
            messages::Type::CreateProcess,
            query_params,
            query_handles
        )?;

        Ok(reply.handle)
    }
}
