use core::fmt;

use hashbrown::HashMap;

use crate::{
    error::{InternalError, ResultExt},
    loader::Loader,
};
use alloc::sync::Arc;
use libruntime::{
    ipc::{self, buffer::BufferView, KHandles},
    kobject::{self, KObject},
    process::{messages, KVBlock},
    sync::{spin::OnceLock, RwLock},
};
use log::{debug, info};

/// Process ID
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
struct Pid(u64);

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Process information stored in the server
#[derive(Debug)]
struct ProcessInfo {
    process: kobject::Process,
    main_thread: kobject::Thread,
    environment: KVBlock,
    arguments: KVBlock,
    exit_code: Option<i32>,
}

/// The main manager structure
#[derive(Debug)]
pub struct Manager {
    processes: RwLock<HashMap<Pid, ProcessInfo>>,
}

impl Manager {
    pub fn new() -> Result<Arc<Self>, kobject::Error> {
        let manager = Manager {
            processes: RwLock::new(HashMap::new()),
        };

        manager.register_init()?;
        manager.register_self()?;

        Ok(Arc::new(manager))
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::Server, kobject::Error> {
        let builder = ipc::ServerBuilder::new(messages::PORT_NAME, messages::VERSION);
        let builder = builder.with_process_exit_handler({
            let manager = Arc::clone(self);
            move |pid| {
                manager.process_terminated(Pid(pid));
            }
        });

        let builder = self.add_handler(
            builder,
            messages::Type::CreateProcess,
            Self::create_process_handler,
        );

        builder.build()
    }

    fn process_terminated(&self, pid: Pid) {}

    fn create_process_handler(
        &self,
        query: messages::CreateProcessQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CreateProcessReply, ipc::KHandles), InternalError> {
        let name_view = {
            let handle =
                query_handles.take(messages::CreateProcessQueryParameters::HANDLE_NAME_MOBJ);
            BufferView::new(handle, &query.name)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let binary_view = {
            let handle =
                query_handles.take(messages::CreateProcessQueryParameters::HANDLE_BINARY_MOBJ);
            BufferView::new(handle, &query.binary)
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
        let pid = Pid(process.pid());

        let info = ProcessInfo {
            process,
            main_thread,
            environment,
            arguments,
            exit_code: None,
        };

        let mut processes = self.processes.write();
        processes.insert(pid, info);

        info!("Created process {}: {}", name, pid);

        Ok((
            messages::CreateProcessReply { handle: 0.into() },
            KHandles::new(),
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
            u64,
        ) -> Result<(ReplyContent, ipc::KHandles), InternalError>,
    ) -> ipc::ServerBuilder
    where
        QueryParameters: Copy + 'static,
        ReplyContent: Copy + 'static,
    {
        let manager = Arc::clone(self);
        builder.with_handler(message_type, move |query, handles, sender_id| {
            handler(&manager, query, handles, sender_id).map_err(|e| e.into_server_error())
        })
    }

    /// Register the process-server itself in the system
    fn register_self(&self) -> Result<(), kobject::Error> {
        let process = kobject::Process::current().clone();
        let main_thread = kobject::Thread::open_self()?;
        let pid = Pid(process.pid());

        let info = ProcessInfo {
            process,
            main_thread,
            environment: Self::get_empty_kvblock(),
            arguments: Self::get_empty_kvblock(),
            exit_code: None,
        };

        let mut processes = self.processes.write();
        processes.insert(pid, info);

        Ok(())
    }

    fn register_init(&self) -> Result<(), kobject::Error> {
        const INIT_PID: u64 = 1;
        // Note: this is fishy, we should really find the main thread differently
        const INIT_MAIN_THREAD_TID: u64 = 3;

        let process = kobject::Process::open(INIT_PID)?;
        let main_thread = kobject::Thread::open(INIT_MAIN_THREAD_TID)?;
        let pid = Pid(process.pid());

        let info = ProcessInfo {
            process,
            main_thread,
            environment: Self::get_empty_kvblock(),
            arguments: Self::get_empty_kvblock(),
            exit_code: None,
        };

        let mut processes = self.processes.write();
        processes.insert(pid, info);

        Ok(())
    }

    fn get_empty_kvblock() -> KVBlock {
        /// Since kvblocks are immutable, we can cache an empty one
        static EMPTY_KVBLOCK: OnceLock<kobject::MemoryObject> = OnceLock::new();

        let mobj = EMPTY_KVBLOCK.get_or_init(|| KVBlock::build(&[]));
        KVBlock::from_memory_object(mobj.clone()).expect("Failed to create KVBlock")
    }
}
