use hashbrown::HashMap;

use crate::{
    error::{InternalError, ResultExt},
    loader::Loader,
    process::{find_live_process, ExitCode, Pid, ProcessInfo},
};
use alloc::{string::String, sync::Arc};
use libruntime::{
    ipc,
    kobject::{self, KObject},
    process::{messages, KVBlock},
    sync::{spin::OnceLock, RwLock},
};
use log::{debug, info, warn};

/// The main manager structure
#[derive(Debug)]
pub struct Manager();

impl Manager {
    pub fn new() -> Result<Arc<Self>, kobject::Error> {
        let manager = Self();

        manager.register_init()?;
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

        builder.build()
    }

    fn process_terminated(&self, pid: Pid) {
        let Some(info) = find_live_process(pid) else {
            warn!("Unknown process with PID {} terminated", pid);
            return;
        };

        info.mark_terminated();

        // TODO: close handles
    }

    fn get_startup_info_handler(
        &self,
        query: messages::GetStartupInfoQueryParameters,
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
            ipc::BufferView::new(handle, &query.name)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let new_name = String::from(unsafe { buffer_view.str() });

        info.update_name(new_name);

        let reply = messages::UpdateNameReply {};
        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }

    fn update_env_handler(
        &self,
        query: messages::UpdateEnvQueryParameters,
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
            ipc::BufferView::new(handle, &query.name)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let binary_view = {
            let handle =
                query_handles.take(messages::CreateProcessQueryParameters::HANDLE_BINARY_MOBJ);
            ipc::BufferView::new(handle, &query.binary)
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

        // Create associated ProcessInfo
        let info = ProcessInfo::new(
            process,
            main_thread,
            String::from(name),
            environment,
            arguments,
        );

        Ok((
            messages::CreateProcessReply { handle: 0.into() },
            ipc::KHandles::new(),
        ))
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

    /// Register the process-server itself in the system
    fn register_self(&self) -> Result<(), kobject::Error> {
        let process = kobject::Process::current().clone();
        let main_thread = kobject::Thread::open_self()?;

        ProcessInfo::new(
            process,
            main_thread,
            String::from("process-server"),
            Self::get_empty_kvblock(),
            Self::get_empty_kvblock(),
        );

        Ok(())
    }

    fn register_init(&self) -> Result<(), kobject::Error> {
        const INIT_PID: u64 = 1;
        // Note: this is fishy, we should really find the main thread differently
        const INIT_MAIN_THREAD_TID: u64 = 3;

        let process = kobject::Process::open(INIT_PID)?;
        let main_thread = kobject::Thread::open(INIT_MAIN_THREAD_TID)?;

        ProcessInfo::new(
            process,
            main_thread,
            String::from("init"),
            Self::get_empty_kvblock(),
            Self::get_empty_kvblock(),
        );

        Ok(())
    }

    fn get_empty_kvblock() -> KVBlock {
        /// Since kvblocks are immutable, we can cache an empty one
        static EMPTY_KVBLOCK: OnceLock<kobject::MemoryObject> = OnceLock::new();

        let mobj = EMPTY_KVBLOCK.get_or_init(|| KVBlock::build(&[]));
        KVBlock::from_memory_object(mobj.clone()).expect("Failed to create KVBlock")
    }
}
