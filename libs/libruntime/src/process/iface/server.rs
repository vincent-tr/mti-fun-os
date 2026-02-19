use core::fmt;

use crate::{
    ipc,
    kobject::{self, KObject},
};
use alloc::{string::String, sync::Arc, vec::Vec};
use log::error;

use super::{
    messages, KVBlock, ProcessInfo, ProcessListBlock, ProcessServerError, ProcessStatus,
    StartupInfo,
};

/// Process server interface
pub trait ProcessServer {
    type Error: Into<ProcessServerError>;

    fn process_terminated(&self, _pid: u64) {}

    fn get_startup_info(&self, sender_id: u64) -> Result<StartupInfo, Self::Error>;

    fn update_name(&self, sender_id: u64, new_name: &str) -> Result<(), Self::Error>;

    fn update_env(&self, sender_id: u64, new_env: KVBlock) -> Result<(), Self::Error>;

    fn set_exit_code(&self, sender_id: u64, exit_code: i32) -> Result<(), Self::Error>;

    fn create_process(
        &self,
        sender_id: u64,
        name: &str,
        binary: &[u8],
        environment: KVBlock,
        arguments: KVBlock,
    ) -> Result<(ipc::Handle, u64), Self::Error>;

    fn open_process(&self, sender_id: u64, pid: u64) -> Result<(ipc::Handle, u64), Self::Error>;

    fn close_process(&self, sender_id: u64, handle: ipc::Handle) -> Result<(), Self::Error>;

    fn get_process_name(&self, sender_id: u64, handle: ipc::Handle) -> Result<String, Self::Error>;

    fn get_process_env(&self, sender_id: u64, handle: ipc::Handle) -> Result<KVBlock, Self::Error>;

    fn get_process_args(&self, sender_id: u64, handle: ipc::Handle)
        -> Result<KVBlock, Self::Error>;

    fn get_process_status(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
    ) -> Result<ProcessStatus, Self::Error>;

    fn terminate_process(&self, sender_id: u64, handle: ipc::Handle) -> Result<(), Self::Error>;

    fn list_processes(&self, sender_id: u64) -> Result<Vec<ProcessInfo>, Self::Error>;

    fn register_process_terminated_notification(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
        port: kobject::PortSender,
        correlation: u64,
    ) -> Result<ipc::Handle, Self::Error>;

    fn unregister_process_terminated_notification(
        &self,
        sender_id: u64,
        registration_handle: ipc::Handle,
    ) -> Result<(), Self::Error>;
}

/// The main server structure
#[derive(Debug)]
pub struct Server<Impl: ProcessServer + 'static> {
    inner: Impl,
}

impl<Impl: ProcessServer + 'static> Server<Impl> {
    pub fn new(inner: Impl) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::Server, kobject::Error> {
        let builder = ipc::ManagedServerBuilder::<_, ProcessServerError, ProcessServerError>::new(
            &self,
            messages::PORT_NAME,
            messages::VERSION,
        );
        let builder = builder.with_process_exit_handler(Self::process_terminated_handler);

        let builder = builder.with_handler(
            messages::Type::GetStartupInfo,
            Self::get_startup_info_handler,
        );
        let builder = builder.with_handler(messages::Type::UpdateName, Self::update_name_handler);
        let builder = builder.with_handler(messages::Type::UpdateEnv, Self::update_env_handler);
        let builder =
            builder.with_handler(messages::Type::SetExitCode, Self::set_exit_code_handler);

        let builder =
            builder.with_handler(messages::Type::CreateProcess, Self::create_process_handler);
        let builder = builder.with_handler(messages::Type::OpenProcess, Self::open_process_handler);
        let builder =
            builder.with_handler(messages::Type::CloseProcess, Self::close_process_handler);
        let builder = builder.with_handler(
            messages::Type::GetProcessName,
            Self::get_process_name_handler,
        );
        let builder =
            builder.with_handler(messages::Type::GetProcessEnv, Self::get_process_env_handler);
        let builder = builder.with_handler(
            messages::Type::GetProcessArgs,
            Self::get_process_args_handler,
        );
        let builder = builder.with_handler(
            messages::Type::GetProcessStatus,
            Self::get_process_status_handler,
        );
        let builder = builder.with_handler(
            messages::Type::TerminateProcess,
            Self::terminate_process_handler,
        );
        let builder =
            builder.with_handler(messages::Type::ListProcesses, Self::list_processes_handler);
        let builder = builder.with_handler(
            messages::Type::RegisterProcessTerminatedNotification,
            Self::register_process_terminated_notification_handler,
        );
        let builder = builder.with_handler(
            messages::Type::UnregisterProcessTerminatedNotification,
            Self::unregister_process_terminated_notification_handler,
        );

        builder.build()
    }

    fn process_terminated_handler(&self, pid: u64) {
        self.inner.process_terminated(pid);
    }

    fn get_startup_info_handler(
        &self,
        _query: messages::GetStartupInfoQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetStartupInfoReply, ipc::KHandles), ProcessServerError> {
        let info = self.inner.get_startup_info(sender_id).map_err(Into::into)?;

        let (name_mobj, name_buffer) = ipc::Buffer::new_local(info.name.as_bytes()).into_shared();
        let env_mobj = info.env.into_memory_object();
        let args_mobj = info.args.into_memory_object();
        let symbols_mobj = info.symbols.into_memory_object();

        let mut reply_handles = ipc::KHandles::new();
        reply_handles[messages::GetStartupInfoReply::HANDLE_NAME_MOBJ] = name_mobj.into_handle();
        reply_handles[messages::GetStartupInfoReply::HANDLE_ENV_MOBJ] = env_mobj.into_handle();
        reply_handles[messages::GetStartupInfoReply::HANDLE_ARGS_MOBJ] = args_mobj.into_handle();
        reply_handles[messages::GetStartupInfoReply::HANDLE_SYMBOLS_MOBJ] =
            symbols_mobj.into_handle();

        Ok((
            messages::GetStartupInfoReply { name: name_buffer },
            reply_handles,
        ))
    }

    fn update_name_handler(
        &self,
        query: messages::UpdateNameQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::UpdateNameReply, ipc::KHandles), ProcessServerError> {
        let buffer_view = {
            let handle = query_handles.take(messages::UpdateNameQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let new_name = unsafe { buffer_view.str() };

        self.inner
            .update_name(sender_id, new_name)
            .map_err(Into::into)?;

        Ok((messages::UpdateNameReply {}, ipc::KHandles::new()))
    }

    fn update_env_handler(
        &self,
        _query: messages::UpdateEnvQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::UpdateEnvReply, ipc::KHandles), ProcessServerError> {
        let new_env = {
            let mobj = kobject::MemoryObject::from_handle(
                query_handles.take(messages::UpdateEnvQueryParameters::HANDLE_ENV_MOBJ),
            )
            .invalid_arg("Bad handle for environment kvblock")?;
            KVBlock::from_memory_object(mobj).invalid_arg("Failed to load environment kvblock")?
        };

        self.inner
            .update_env(sender_id, new_env)
            .map_err(Into::into)?;

        Ok((messages::UpdateEnvReply {}, ipc::KHandles::new()))
    }

    fn set_exit_code_handler(
        &self,
        query: messages::SetExitCodeQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::SetExitCodeReply, ipc::KHandles), ProcessServerError> {
        self.inner
            .set_exit_code(sender_id, query.exit_code)
            .map_err(Into::into)?;

        Ok((messages::SetExitCodeReply {}, ipc::KHandles::new()))
    }

    fn create_process_handler(
        &self,
        query: messages::CreateProcessQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CreateProcessReply, ipc::KHandles), ProcessServerError> {
        let name_view = {
            let handle =
                query_handles.take(messages::CreateProcessQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let binary_view = {
            let handle =
                query_handles.take(messages::CreateProcessQueryParameters::HANDLE_BINARY_MOBJ);
            ipc::BufferView::new(handle, &query.binary, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create binary buffer reader")?
        };

        let name = unsafe { name_view.str() };
        let binary = binary_view.buffer();

        // Validate kvblocks
        let environment = {
            let mobj = kobject::MemoryObject::from_handle(
                query_handles.take(messages::CreateProcessQueryParameters::HANDLE_ENV_MOBJ),
            )
            .invalid_arg("Bad handle for environment kvblock")?;
            KVBlock::from_memory_object(mobj).invalid_arg("Failed to load environment kvblock")?
        };

        let arguments = {
            let mobj = kobject::MemoryObject::from_handle(
                query_handles.take(messages::CreateProcessQueryParameters::HANDLE_ARGS_MOBJ),
            )
            .invalid_arg("Bad handle for arguments kvblock")?;
            KVBlock::from_memory_object(mobj).invalid_arg("Failed to load arguments kvblock")?
        };

        let (handle, pid) = self
            .inner
            .create_process(sender_id, name, binary, environment, arguments)
            .map_err(Into::into)?;

        Ok((
            messages::CreateProcessReply { handle, pid },
            ipc::KHandles::new(),
        ))
    }

    fn open_process_handler(
        &self,
        query: messages::OpenProcessQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::OpenProcessReply, ipc::KHandles), ProcessServerError> {
        let (handle, pid) = self
            .inner
            .open_process(sender_id, query.pid)
            .map_err(Into::into)?;

        Ok((
            messages::OpenProcessReply { handle, pid },
            ipc::KHandles::new(),
        ))
    }

    fn close_process_handler(
        &self,
        query: messages::CloseProcessQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CloseProcessReply, ipc::KHandles), ProcessServerError> {
        self.inner
            .close_process(sender_id, query.handle)
            .map_err(Into::into)?;

        Ok((messages::CloseProcessReply {}, ipc::KHandles::new()))
    }

    fn get_process_name_handler(
        &self,
        query: messages::GetProcessNameQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetProcessNameReply, ipc::KHandles), ProcessServerError> {
        let name = self
            .inner
            .get_process_name(sender_id, query.handle)
            .map_err(Into::into)?;

        let mut name_view = {
            let handle =
                query_handles.take(messages::GetProcessNameQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create name buffer reader")?
        };

        if name.len() > name_view.buffer().len() {
            log::error!(
                "Provided buffer too small for process name ({} bytes needed, {} bytes provided)",
                name.len(),
                name_view.buffer().len()
            );
            return Err(ProcessServerError::BufferTooSmall);
        }

        name_view.buffer_mut()[..name.len()].copy_from_slice(name.as_bytes());

        Ok((
            messages::GetProcessNameReply {
                buffer_used_len: name.len(),
            },
            ipc::KHandles::new(),
        ))
    }

    fn get_process_env_handler(
        &self,
        query: messages::GetProcessEnvQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetProcessEnvReply, ipc::KHandles), ProcessServerError> {
        let env = self
            .inner
            .get_process_env(sender_id, query.handle)
            .map_err(Into::into)?;

        let mut reply_handles = ipc::KHandles::new();
        reply_handles[messages::GetProcessEnvReply::HANDLE_ENV_MOBJ] =
            env.into_memory_object().into_handle();

        Ok((messages::GetProcessEnvReply {}, reply_handles))
    }

    fn get_process_args_handler(
        &self,
        query: messages::GetProcessArgsQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetProcessArgsReply, ipc::KHandles), ProcessServerError> {
        let args = self
            .inner
            .get_process_args(sender_id, query.handle)
            .map_err(Into::into)?;

        let mut reply_handles = ipc::KHandles::new();
        reply_handles[messages::GetProcessArgsReply::HANDLE_ARGS_MOBJ] =
            args.into_memory_object().into_handle();

        Ok((messages::GetProcessArgsReply {}, reply_handles))
    }

    fn get_process_status_handler(
        &self,
        query: messages::GetProcessStatusQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetProcessStatusReply, ipc::KHandles), ProcessServerError> {
        let status = self
            .inner
            .get_process_status(sender_id, query.handle)
            .map_err(Into::into)?;

        Ok((
            messages::GetProcessStatusReply { status },
            ipc::KHandles::new(),
        ))
    }

    fn terminate_process_handler(
        &self,
        query: messages::TerminateProcessQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::TerminateProcessReply, ipc::KHandles), ProcessServerError> {
        self.inner
            .terminate_process(sender_id, query.handle)
            .map_err(Into::into)?;

        Ok((messages::TerminateProcessReply {}, ipc::KHandles::new()))
    }

    fn list_processes_handler(
        &self,
        query: messages::ListProcessesQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListProcessesReply, ipc::KHandles), ProcessServerError> {
        let mut buffer_view = {
            let handle =
                query_handles.take(messages::ListProcessesQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create process list buffer view")?
        };

        let processes: Vec<_> = self.inner.list_processes(sender_id).map_err(Into::into)?;

        let buffer_used_len =
            ProcessListBlock::build(&processes, buffer_view.buffer_mut()).map_err(
                |required_size| {
                    error!("Provided buffer too small for process list ({} bytes needed, {} bytes provided)",
                        required_size,
                        buffer_view.buffer().len()
                    );
                    ProcessServerError::BufferTooSmall
                },
            )?;

        Ok((
            messages::ListProcessesReply { buffer_used_len },
            ipc::KHandles::new(),
        ))
    }

    fn register_process_terminated_notification_handler(
        &self,
        query: messages::RegisterProcessTerminatedNotificationQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<
        (
            messages::RegisterProcessTerminatedNotificationReply,
            ipc::KHandles,
        ),
        ProcessServerError,
    > {
        let port = {
            let handle = query_handles
                .take(messages::RegisterProcessTerminatedNotificationQueryParameters::HANDLE_PORT);
            kobject::PortSender::from_handle(handle)
                .invalid_arg("Failed to create port from handle")?
        };

        let registration_handle = self
            .inner
            .register_process_terminated_notification(
                sender_id,
                query.handle,
                port,
                query.correlation,
            )
            .map_err(Into::into)?;

        Ok((
            messages::RegisterProcessTerminatedNotificationReply {
                registration_handle,
            },
            ipc::KHandles::new(),
        ))
    }

    fn unregister_process_terminated_notification_handler(
        &self,
        query: messages::UnregisterProcessTerminatedNotificationQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<
        (
            messages::UnregisterProcessTerminatedNotificationReply,
            ipc::KHandles,
        ),
        ProcessServerError,
    > {
        self.inner
            .unregister_process_terminated_notification(sender_id, query.registration_handle)
            .map_err(Into::into)?;

        Ok((
            messages::UnregisterProcessTerminatedNotificationReply {},
            ipc::KHandles::new(),
        ))
    }
}

/// Extension trait for Result to add context
trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, ProcessServerError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, ProcessServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            ProcessServerError::InvalidArgument
        })
    }
}
