use alloc::string::String;

use super::{messages, KVBlock};
use crate::{
    ipc,
    kobject::{self, KObject},
};

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

    /// call ipc CreateProcess
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
            query_handles,
        )?;

        Ok(reply.handle)
    }

    /// call ipc GetStartupInfo
    pub fn get_startup_info(
        &self,
    ) -> Result<StartupInfo, ipc::CallError<messages::ProcessServerError>> {
        let query = messages::GetStartupInfoQueryParameters {};
        let query_handles = ipc::KHandles::new();

        let (reply, mut reply_handles) = self.ipc_client.call::<messages::Type, messages::GetStartupInfoQueryParameters, messages::GetStartupInfoReply, messages::ProcessServerError>(
            messages::Type::GetStartupInfo,
            query,
            query_handles,
        )?;

        let name = {
            let handle = reply_handles.take(messages::GetStartupInfoReply::HANDLE_NAME_MOBJ);
            let buffer_view =
                ipc::BufferView::new(handle, &reply.name).expect("could not read name");
            String::from(unsafe { buffer_view.str() })
        };

        let env = {
            let handle = reply_handles.take(messages::GetStartupInfoReply::HANDLE_ENV_MOBJ);
            let mobj = kobject::MemoryObject::from_handle(handle)
                .expect("could not get env memory object");
            KVBlock::from_memory_object(mobj).expect("could not read KVBlock")
        };

        let args = {
            let handle = reply_handles.take(messages::GetStartupInfoReply::HANDLE_ARGS_MOBJ);
            let mobj = kobject::MemoryObject::from_handle(handle)
                .expect("could not get args memory object");
            KVBlock::from_memory_object(mobj).expect("could not read KVBlock")
        };

        Ok(StartupInfo { name, env, args })
    }

    /// call ipc UpdateName
    pub fn update_name(
        &self,
        name: &str,
    ) -> Result<(), ipc::CallError<messages::ProcessServerError>> {
        let (name_memobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::UpdateNameQueryParameters { name: name_buffer };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::UpdateNameQueryParameters::HANDLE_NAME_MOBJ] =
            name_memobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::UpdateNameQueryParameters, messages::UpdateNameReply, messages::ProcessServerError>(
            messages::Type::UpdateName,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc UpdateEnv
    pub fn update_env(
        &self,
        env_memobj: kobject::MemoryObject,
    ) -> Result<(), ipc::CallError<messages::ProcessServerError>> {
        let env_memobj = env_memobj.clone().into_handle();

        let query = messages::UpdateEnvQueryParameters {};

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::UpdateEnvQueryParameters::HANDLE_ENV_MOBJ] = env_memobj;

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::UpdateEnvQueryParameters, messages::UpdateEnvReply, messages::ProcessServerError>(
            messages::Type::UpdateEnv,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc SetExitCode
    pub fn set_exit_code(
        &self,
        code: i32,
    ) -> Result<(), ipc::CallError<messages::ProcessServerError>> {
        let query = messages::SetExitCodeQueryParameters { exit_code: code };

        let query_handles = ipc::KHandles::new();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::SetExitCodeQueryParameters, messages::SetExitCodeReply, messages::ProcessServerError>(
            messages::Type::SetExitCode,
            query,
            query_handles,
        )?;

        Ok(())
    }
}

/// Process startup information.
#[derive(Debug)]
pub struct StartupInfo {
    pub name: String,
    pub env: KVBlock,
    pub args: KVBlock,
}
