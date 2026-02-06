use alloc::{string::String, vec::Vec};

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
                ipc::BufferView::new(handle, &reply.name, ipc::BufferViewAccess::ReadOnly)
                    .expect("could not read name");
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

    /// call ipc CreateProcess
    pub fn create_process(
        &self,
        name: &str,
        binary: ipc::Buffer<'_>,
        env: &[(&str, &str)],
        args: &[(&str, &str)],
    ) -> Result<(ipc::Handle, u64), ipc::CallError<messages::ProcessServerError>> {
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

        Ok((reply.handle, reply.pid))
    }

    /// call ipc OpenProcess
    pub fn open_process(
        &self,
        pid: u64,
    ) -> Result<(ipc::Handle, u64), ipc::CallError<messages::ProcessServerError>> {
        let query = messages::OpenProcessQueryParameters { pid };

        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::OpenProcessQueryParameters, messages::OpenProcessReply, messages::ProcessServerError>(
            messages::Type::OpenProcess,
            query,
            query_handles,
        )?;

        Ok((reply.handle, reply.pid))
    }

    /// call ipc CloseProcess
    pub fn close_process(
        &self,
        handle: ipc::Handle,
    ) -> Result<(), ipc::CallError<messages::ProcessServerError>> {
        let query = messages::CloseProcessQueryParameters { handle };

        let query_handles = ipc::KHandles::new();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::CloseProcessQueryParameters, messages::CloseProcessReply, messages::ProcessServerError>(
            messages::Type::CloseProcess,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc GetProcessName
    pub fn get_process_name(
        &self,
        handle: ipc::Handle,
    ) -> Result<String, ipc::CallError<messages::ProcessServerError>> {
        // The process name can be of arbitrary length, so we start with a small buffer and increase it until it's large enough to hold the name.
        let mut size = 64;

        let allocated_buffer = loop {
            let mut allocated_buffer = {
                let mut vec = Vec::with_capacity(size);
                unsafe { vec.set_len(size) };
                vec
            };

            let (buffer_mobj, buffer) = ipc::Buffer::new_local(&allocated_buffer).into_shared();

            let query = messages::GetProcessNameQueryParameters {
                handle,
                name: buffer,
            };

            let mut query_handles = ipc::KHandles::new();
            query_handles[messages::GetProcessNameQueryParameters::HANDLE_NAME_MOBJ] =
                buffer_mobj.into_handle();

            let res = self.ipc_client.call::<messages::Type, messages::GetProcessNameQueryParameters, messages::GetProcessNameReply, messages::ProcessServerError>(
                messages::Type::GetProcessName,
                query,
                query_handles,
            );

            if let Err(ipc::CallError::ReplyError(messages::ProcessServerError::BufferTooSmall)) =
                res
            {
                size *= 2;
                continue;
            }

            let (reply, _reply_handles) = res?;

            unsafe { allocated_buffer.set_len(reply.name_len) };
            break allocated_buffer;
        };

        let name = unsafe { String::from_utf8_unchecked(allocated_buffer) };

        Ok(name)
    }

    pub fn get_process_env(
        &self,
        handle: ipc::Handle,
    ) -> Result<KVBlock, ipc::CallError<messages::ProcessServerError>> {
        let query = messages::GetProcessEnvQueryParameters { handle };

        let query_handles = ipc::KHandles::new();

        let (_reply, mut reply_handles) = self.ipc_client.call::<messages::Type, messages::GetProcessEnvQueryParameters, messages::GetProcessEnvReply, messages::ProcessServerError>(
            messages::Type::GetProcessEnv,
            query,
            query_handles,
        )?;

        let mobj = {
            let handle = reply_handles.take(messages::GetProcessEnvReply::HANDLE_ENV_MOBJ);
            kobject::MemoryObject::from_handle(handle).expect("could not get env memory object")
        };

        let env = KVBlock::from_memory_object(mobj).expect("could not read KVBlock");

        Ok(env)
    }

    /// call ipc GetProcessArgs
    pub fn get_process_args(
        &self,
        handle: ipc::Handle,
    ) -> Result<KVBlock, ipc::CallError<messages::ProcessServerError>> {
        let query = messages::GetProcessArgsQueryParameters { handle };

        let query_handles = ipc::KHandles::new();

        let (_reply, mut reply_handles) = self.ipc_client.call::<messages::Type, messages::GetProcessArgsQueryParameters, messages::GetProcessArgsReply, messages::ProcessServerError>(
            messages::Type::GetProcessArgs,
            query,
            query_handles,
        )?;

        let mobj = {
            let handle = reply_handles.take(messages::GetProcessArgsReply::HANDLE_ARGS_MOBJ);
            kobject::MemoryObject::from_handle(handle).expect("could not get args memory object")
        };

        let args = KVBlock::from_memory_object(mobj).expect("could not read KVBlock");

        Ok(args)
    }

    /// call ipc GetProcessStatus
    pub fn get_process_status(
        &self,
        handle: ipc::Handle,
    ) -> Result<messages::ProcessStatus, ipc::CallError<messages::ProcessServerError>> {
        let query = messages::GetProcessStatusQueryParameters { handle };

        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::GetProcessStatusQueryParameters, messages::GetProcessStatusReply, messages::ProcessServerError>(
            messages::Type::GetProcessStatus,
            query,
            query_handles,
        )?;

        Ok(reply.status)
    }

    /// call ipc TerminateProcess
    pub fn terminate_process(
        &self,
        handle: ipc::Handle,
    ) -> Result<(), ipc::CallError<messages::ProcessServerError>> {
        let query = messages::TerminateProcessQueryParameters { handle };

        let query_handles = ipc::KHandles::new();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::TerminateProcessQueryParameters, messages::TerminateProcessReply, messages::ProcessServerError>(
            messages::Type::TerminateProcess,
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
