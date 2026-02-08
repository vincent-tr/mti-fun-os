use crate::{
    error::{InternalError, ResultExt},
    loader::Loader,
    process::{
        find_live_process, find_process, get_termination_registration_by_handle, list_processes,
        list_termination_registrations_by_owner, ExitCode, Pid, ProcessInfo,
        TerminationRegistration,
    },
    state::State,
};
use alloc::{string::String, sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use libruntime::{
    ipc,
    kobject::{self, KObject},
    process::{self, messages, KVBlock},
};
use log::{debug, info, warn};

/// The main manager structure
#[derive(Debug)]
pub struct Manager {
    handles: ipc::HandleTable<'static, ProcessInfo>,
    handle_generator: &'static ipc::HandleGenerator,
}

impl Manager {
    pub fn new() -> Result<Arc<Self>, kobject::Error> {
        let state = State::get();
        let handle_generator = state.handle_generator();
        let handles = ipc::HandleTable::new(handle_generator);

        let manager = Self {
            handles,
            handle_generator,
        };

        manager.register_init()?;
        manager.register_idle()?;
        manager.register_self()?;

        Ok(Arc::new(manager))
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::Server, kobject::Error> {
        let builder = ipc::ServerBuilder::new(messages::PORT_NAME, messages::VERSION);
        let builder = builder.with_process_exit_handler({
            let manager = Arc::clone(self);
            move |pid| {
                manager.process_terminated(Pid::from(pid));
            }
        });

        let builder = self.add_handler(
            builder,
            messages::Type::GetStartupInfo,
            Self::get_startup_info_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::UpdateName,
            Self::update_name_handler,
        );
        let builder =
            self.add_handler(builder, messages::Type::UpdateEnv, Self::update_env_handler);
        let builder = self.add_handler(
            builder,
            messages::Type::SetExitCode,
            Self::set_exit_code_handler,
        );

        let builder = self.add_handler(
            builder,
            messages::Type::CreateProcess,
            Self::create_process_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::OpenProcess,
            Self::open_process_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::CloseProcess,
            Self::close_process_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::GetProcessName,
            Self::get_process_name_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::GetProcessEnv,
            Self::get_process_env_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::GetProcessArgs,
            Self::get_process_args_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::GetProcessStatus,
            Self::get_process_status_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::TerminateProcess,
            Self::terminate_process_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::ListProcesses,
            Self::list_processes_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::RegisterProcessTerminatedNotification,
            Self::register_process_terminated_notification_handler,
        );
        let builder = self.add_handler(
            builder,
            messages::Type::UnregisterProcessTerminatedNotification,
            Self::unregister_process_terminated_notification_handler,
        );

        builder.build()
    }

    fn process_terminated(&self, pid: Pid) {
        let Some(info) = find_live_process(pid) else {
            warn!("Unknown process with PID {} terminated", pid);
            return;
        };

        info.mark_terminated();

        self.handles.process_terminated(pid.as_u64());

        for (target_pid, owner_handle) in list_termination_registrations_by_owner(pid) {
            let process = find_process(pid).expect("failed to get registration process");
            process.remove_termination_registration(owner_handle);
        }
    }

    fn get_startup_info_handler(
        &self,
        _query: messages::GetStartupInfoQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::GetStartupInfoReply, ipc::KHandles), InternalError> {
        let info = find_live_process(sender_id)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let (name_mobj, name_buffer) = ipc::Buffer::new_local(info.name().as_bytes()).into_shared();
        let env_mobj = info.environment().memory_object().clone();
        let args_mobj = info.arguments().memory_object().clone();

        let reply = messages::GetStartupInfoReply { name: name_buffer };

        let mut reply_handles = ipc::KHandles::new();
        reply_handles[messages::GetStartupInfoReply::HANDLE_NAME_MOBJ] = name_mobj.into_handle();
        reply_handles[messages::GetStartupInfoReply::HANDLE_ENV_MOBJ] = env_mobj.into_handle();
        reply_handles[messages::GetStartupInfoReply::HANDLE_ARGS_MOBJ] = args_mobj.into_handle();

        Ok((reply, reply_handles))
    }

    fn update_name_handler(
        &self,
        query: messages::UpdateNameQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::UpdateNameReply, ipc::KHandles), InternalError> {
        let info = find_live_process(sender_id)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let buffer_view = {
            let handle = query_handles.take(messages::UpdateNameQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let new_name = String::from(unsafe { buffer_view.str() });

        info.kobject_process().set_name(&new_name)?;
        info.update_name(new_name);

        let reply = messages::UpdateNameReply {};
        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn update_env_handler(
        &self,
        _query: messages::UpdateEnvQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::UpdateEnvReply, ipc::KHandles), InternalError> {
        let info = find_live_process(sender_id)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let new_env = {
            let mobj = kobject::MemoryObject::from_handle(
                query_handles.take(messages::UpdateEnvQueryParameters::HANDLE_ENV_MOBJ),
            )
            .invalid_arg("Bad handle for environment kvblock")?;
            KVBlock::from_memory_object(mobj).invalid_arg("Failed to load environment kvblock")?
        };

        info.update_environment(new_env);

        let reply = messages::UpdateEnvReply {};
        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn set_exit_code_handler(
        &self,
        query: messages::SetExitCodeQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::SetExitCodeReply, ipc::KHandles), InternalError> {
        let info = find_live_process(sender_id)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        info.set_exit_code(ExitCode::try_from(query.exit_code).invalid_arg("Invalid exit code")?);

        let reply = messages::SetExitCodeReply {};
        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn create_process_handler(
        &self,
        query: messages::CreateProcessQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::CreateProcessReply, ipc::KHandles), InternalError> {
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

        info!("Creating process {}", name);

        let loader = Loader::new(binary)?;

        let process = kobject::Process::create(name).runtime_err("Failed to create process")?;

        let mappings = loader.map(&process)?;

        // Set up the process's main thread
        let stack_size = kobject::helpers::STACK_SIZE;
        let entry_point = loader.entry_point();
        let stack = kobject::helpers::AllocWithGuards::new_remote(stack_size, &process)
            .runtime_err("Failed to allocated stack")?;
        let tls =
            kobject::helpers::AllocWithGuards::new_remote(kobject::helpers::TLS_SIZE, &process)
                .runtime_err("Failed to allocated TLS block")?;

        let stack_top_addr = stack.address() + stack_size;
        let tls_addr = tls.address();

        debug!(
            "Creating main thread: entry_point={:#x}, stack_top={:#x}, tls={:#x}",
            entry_point as usize, stack_top_addr, tls_addr
        );

        // Use syscall directly to create remote thread
        let main_thread = {
            let thread_handle = libsyscalls::thread::create(
                Some("main"),
                unsafe { process.handle() },
                false,
                kobject::ThreadPriority::Normal,
                entry_point,
                stack_top_addr,
                0, // arg not used
                tls_addr,
            )
            .map_err(|e| kobject::Error::from(e))
            .runtime_err("Failed to create main thread")?;

            unsafe { kobject::Thread::from_handle_unchecked(thread_handle) }
        };

        // Process started, we can leak the allocations
        stack.leak();
        tls.leak();
        for mapping in mappings {
            mapping.leak();
        }

        let pid = process.pid();

        // Create associated ProcessInfo
        let info = ProcessInfo::new(
            sender_id,
            process,
            main_thread,
            String::from(name),
            environment,
            arguments,
        );

        let handle = self.handles.open(sender_id.as_u64(), info);

        Ok((
            messages::CreateProcessReply { handle, pid },
            ipc::KHandles::new(),
        ))
    }

    fn open_process_handler(
        &self,
        query: messages::OpenProcessQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::OpenProcessReply, ipc::KHandles), InternalError> {
        let info = find_process(Pid::from(query.pid))
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let handle = self.handles.open(sender_id.as_u64(), info);

        Ok((
            messages::OpenProcessReply {
                handle,
                pid: query.pid,
            },
            ipc::KHandles::new(),
        ))
    }

    fn close_process_handler(
        &self,
        query: messages::CloseProcessQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::CloseProcessReply, ipc::KHandles), InternalError> {
        self.handles
            .close(sender_id.as_u64(), query.handle)
            .ok_or_else(|| InternalError::invalid_argument("Invalid process handle"))?;

        Ok((messages::CloseProcessReply {}, ipc::KHandles::new()))
    }

    fn get_process_name_handler(
        &self,
        query: messages::GetProcessNameQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::GetProcessNameReply, ipc::KHandles), InternalError> {
        let info = self
            .handles
            .read(sender_id.as_u64(), query.handle)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let mut name_view = {
            let handle =
                query_handles.take(messages::GetProcessNameQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let name = info.name();

        if name.len() > name_view.buffer().len() {
            return Err(InternalError::buffer_too_small(
                "Provided buffer too small for process name",
            ));
        }

        name_view.buffer_mut()[..name.len()].copy_from_slice(name.as_bytes());

        let reply = messages::GetProcessNameReply {
            buffer_used_len: name.len(),
        };

        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn get_process_env_handler(
        &self,
        query: messages::GetProcessEnvQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::GetProcessEnvReply, ipc::KHandles), InternalError> {
        let info = self
            .handles
            .read(sender_id.as_u64(), query.handle)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let env_mobj = info.environment().memory_object().clone();

        let reply = messages::GetProcessEnvReply {};

        let mut reply_handles = ipc::KHandles::new();
        reply_handles[messages::GetProcessEnvReply::HANDLE_ENV_MOBJ] = env_mobj.into_handle();

        Ok((reply, reply_handles))
    }

    fn get_process_args_handler(
        &self,
        query: messages::GetProcessArgsQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::GetProcessArgsReply, ipc::KHandles), InternalError> {
        let info = self
            .handles
            .read(sender_id.as_u64(), query.handle)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let args_mobj = info.arguments().memory_object().clone();

        let reply = messages::GetProcessArgsReply {};

        let mut reply_handles = ipc::KHandles::new();
        reply_handles[messages::GetProcessArgsReply::HANDLE_ARGS_MOBJ] = args_mobj.into_handle();

        Ok((reply, reply_handles))
    }

    fn get_process_status_handler(
        &self,
        query: messages::GetProcessStatusQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::GetProcessStatusReply, ipc::KHandles), InternalError> {
        let info = self
            .handles
            .read(sender_id.as_u64(), query.handle)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let status = if info.is_terminated() {
            messages::ProcessStatus::Exited(info.exit_code().as_i32())
        } else {
            messages::ProcessStatus::Running
        };

        let reply = messages::GetProcessStatusReply { status };

        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn terminate_process_handler(
        &self,
        query: messages::TerminateProcessQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<(messages::TerminateProcessReply, ipc::KHandles), InternalError> {
        let info = self
            .handles
            .read(sender_id.as_u64(), query.handle)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        if info.is_terminated() {
            return Err(InternalError::process_already_terminated(
                "Process already terminated",
            ));
        }

        // Note: this is a bit hacky, we can set the exit code to KILLED and mark the process as terminated, and let the process cleanup itself. This way we don't have to forcefully kill the process from the kernel, which would be more complex and error-prone.
        info.kobject_process()
            .kill()
            .runtime_err("Could not kill process")?;

        info.set_exit_code(ExitCode::KILLED);
        // Note: we will get kernel notifications for the process exit, so we can just mark it as terminated when we receive the notification.
        // This allows the kernel to be the only source of truth for whether a process is alive or not

        let reply = messages::TerminateProcessReply {};

        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn list_processes_handler(
        &self,
        query: messages::ListProcessesQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: Pid,
    ) -> Result<(messages::ListProcessesReply, ipc::KHandles), InternalError> {
        let mut buffer_view = {
            let handle =
                query_handles.take(messages::ListProcessesQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create process list buffer view")?
        };

        let processes: Vec<_> = list_processes()
            .iter()
            .map(|p| process::ProcessInfo {
                pid: p.pid().as_u64(),
                ppid: p.creator().as_u64(),

                name: p.name(),
                status: if p.is_terminated() {
                    messages::ProcessStatus::Exited(p.exit_code().as_i32())
                } else {
                    messages::ProcessStatus::Running
                },
            })
            .collect();

        let buffer_used_len =
            process::ProcessListBlock::build(&processes, buffer_view.buffer_mut()).map_err(
                |_required_size| {
                    InternalError::buffer_too_small("Provided buffer too small for process list")
                },
            )?;

        let reply = messages::ListProcessesReply { buffer_used_len };

        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn register_process_terminated_notification_handler(
        &self,
        query: messages::RegisterProcessTerminatedNotificationQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<
        (
            messages::RegisterProcessTerminatedNotificationReply,
            ipc::KHandles,
        ),
        InternalError,
    > {
        let info = self
            .handles
            .read(sender_id.as_u64(), query.handle)
            .ok_or_else(|| InternalError::invalid_argument("Process not found"))?;

        let port = {
            let handle = query_handles
                .take(messages::RegisterProcessTerminatedNotificationQueryParameters::HANDLE_PORT);
            kobject::PortSender::from_handle(handle)
                .invalid_arg("Failed to create port from handle")?
        };

        let owner_handle = self.handle_generator.generate();
        let registration = TerminationRegistration::new(
            sender_id,
            owner_handle,
            info.clone(),
            port,
            query.correlation,
        );

        info.add_termination_registration(registration);

        let reply = messages::RegisterProcessTerminatedNotificationReply {
            registration_handle: owner_handle,
        };

        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn unregister_process_terminated_notification_handler(
        &self,
        query: messages::UnregisterProcessTerminatedNotificationQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: Pid,
    ) -> Result<
        (
            messages::UnregisterProcessTerminatedNotificationReply,
            ipc::KHandles,
        ),
        InternalError,
    > {
        let (target_pid, owner_handle) =
            get_termination_registration_by_handle(query.registration_handle)
                .ok_or_else(|| InternalError::invalid_argument("Invalid registration handle"))?;

        let process =
            find_process(target_pid).expect("data inconsistency: registration process not found");

        // ensure the sender is the owner of the registration
        if process.get_registration_owner(owner_handle) != sender_id {
            return Err(InternalError::invalid_argument(
                "Sender is not the owner of the registration",
            ));
        }

        process.remove_termination_registration(owner_handle);

        let reply = messages::UnregisterProcessTerminatedNotificationReply {};
        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn add_handler<QueryParameters, ReplyContent>(
        self: &Arc<Self>,
        builder: ipc::ServerBuilder,
        message_type: messages::Type,
        handler: fn(
            &Self,
            QueryParameters,
            ipc::KHandles,
            Pid,
        ) -> Result<(ReplyContent, ipc::KHandles), InternalError>,
    ) -> ipc::ServerBuilder
    where
        QueryParameters: Copy + 'static,
        ReplyContent: Copy + 'static,
    {
        let manager = Arc::clone(self);
        builder.with_handler(message_type, move |query, handles, sender_id| {
            handler(&manager, query, handles, Pid::from(sender_id))
                .map_err(|e| e.into_server_error())
        })
    }

    const INIT_PID: u64 = 1;
    const IDLE_PID: u64 = 2;

    /// Register the init process in the system, so it shows up in process lists.
    fn register_init(&self) -> Result<(), kobject::Error> {
        // Note: this is fishy, we should really find the main thread differently
        const INIT_MAIN_THREAD_TID: u64 = 3;

        let process = kobject::Process::open(Self::INIT_PID)?;
        let main_thread = kobject::Thread::open(INIT_MAIN_THREAD_TID)?;

        assert!(
            process.name().expect("failed to get process name") == "init",
            "PID 1 is expected to be init process"
        );

        assert!(
            main_thread.name().expect("failed to get thread name") == "main",
            "Init process's main thread is expected to be named 'main'"
        );

        ProcessInfo::new(
            Pid::INVALID,
            process,
            main_thread,
            String::from("init"),
            Self::get_empty_kvblock(),
            Self::get_empty_kvblock(),
        );

        Ok(())
    }

    /// Register the idle process in the system, so it shows up in process lists.
    fn register_idle(&self) -> Result<(), kobject::Error> {
        // Note: this is fishy, we should really find the idle thread differently
        const IDLE_MAIN_THREAD_TID: u64 = 4;

        let process = kobject::Process::open(Self::IDLE_PID)?;
        let main_thread = kobject::Thread::open(IDLE_MAIN_THREAD_TID)?;

        assert!(
            process.name().expect("failed to get process name") == "idle",
            "PID 2 is expected to be idle process"
        );

        assert!(
            main_thread.name().expect("failed to get thread name") == "idle",
            "Idle process's main thread is expected to be named 'idle'"
        );

        ProcessInfo::new(
            Pid::from(Self::INIT_PID),
            process,
            main_thread,
            String::from("idle"),
            Self::get_empty_kvblock(),
            Self::get_empty_kvblock(),
        );

        Ok(())
    }

    /// Register the process-server itself in the system
    fn register_self(&self) -> Result<(), kobject::Error> {
        let process = kobject::Process::current().clone();
        let main_thread = kobject::Thread::open_self()?;

        ProcessInfo::new(
            Pid::from(Self::INIT_PID),
            process,
            main_thread,
            String::from("process-server"),
            Self::get_empty_kvblock(),
            Self::get_empty_kvblock(),
        );

        Ok(())
    }

    fn get_empty_kvblock() -> KVBlock {
        // Since kvblocks are immutable, we can cache an empty one
        lazy_static! {
            static ref EMPTY_KVBLOCK: kobject::MemoryObject = KVBlock::build(&[]);
        }

        KVBlock::from_memory_object(EMPTY_KVBLOCK.clone()).expect("Failed to create KVBlock")
    }
}
