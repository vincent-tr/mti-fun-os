use core::{error, mem::MaybeUninit};

use hashbrown::HashMap;

use alloc::sync::Arc;
use libruntime::{
    ipc::{self, buffer::BufferReader},
    kobject,
    process::{
        messages::{self, ProcessServerError},
        KVBlock,
    },
    sync::{spin::OnceLock, RwLock},
};
use log::{error, info};

/// Process ID
#[derive(Debug, Eq, Hash, PartialEq)]
struct Pid(u64);

/// Process information stored in the server
#[derive(Debug)]
struct ProcessInfo {
    process: kobject::Process,
    main_thread: kobject::Thread,
    environment: kobject::MemoryObject,
    arguments: kobject::MemoryObject,
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
        let builder =
            self.add_handler(builder, messages::Type::CreateProcess, Self::create_process);
        builder.build()
    }

    fn add_handler<QueryParameters, ReplyContent>(
        self: &Arc<Self>,
        builder: ipc::ServerBuilder,
        message_type: messages::Type,
        handler: fn(
            &Self,
            QueryParameters,
            ipc::Handles,
        ) -> Result<(ReplyContent, ipc::Handles), ProcessServerError>,
    ) -> ipc::ServerBuilder
    where
        QueryParameters: Copy + 'static,
        ReplyContent: Copy + 'static,
    {
        let manager = Arc::clone(self);
        builder.with_handler(message_type, move |query, handles| {
            handler(&manager, query, handles)
        })
    }

    fn create_process(
        &self,
        query: messages::CreateProcessQueryParameters,
        mut query_handles: ipc::Handles,
    ) -> Result<(messages::CreateProcessReply, ipc::Handles), ProcessServerError> {
        let name_handle =
            query_handles.take(messages::CreateProcessQueryParameters::HANDLE_NAME_MOBJ);

        let name_reader = BufferReader::new(name_handle, &query.name).map_err(|err| {
            error!("failed to create name buffer reader: {:?}", err);
            ProcessServerError::InvalidArgument
        })?;
        let str = unsafe { str::from_utf8_unchecked(name_reader.buffer()) };

        info!("Creating process {}", str);

        Err(ProcessServerError::InvalidArgument)
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

    fn get_empty_kvblock() -> kobject::MemoryObject {
        /// Since kvblocks are immutable, we can cache an empty one
        static EMPTY_KVBLOCK: OnceLock<kobject::MemoryObject> = OnceLock::new();

        let mobj = EMPTY_KVBLOCK.get_or_init(|| KVBlock::build(&[]));
        mobj.clone()
    }
}
