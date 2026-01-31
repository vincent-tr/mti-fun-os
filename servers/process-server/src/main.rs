#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;
extern crate libruntime;

use hashbrown::HashMap;

use log::{error, info};

use libruntime::{
    ipc::{self, buffer::BufferReader},
    kobject::{self, KObject, Permissions},
    memory,
    process::{messages, KVBlock},
    sync::{spin::OnceLock, RwLock},
};

lazy_static::lazy_static! {
    static ref PROCESSES: RwLock<HashMap<Pid, ProcessInfo>> = RwLock::new(HashMap::new());
}

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

fn get_empty_kvblock() -> kobject::MemoryObject {
    /// Since kvblocks are immutable, we can cache an empty one
    static EMPTY_KVBLOCK: OnceLock<kobject::MemoryObject> = OnceLock::new();

    let mobj = EMPTY_KVBLOCK.get_or_init(|| KVBlock::build(&[]));
    mobj.clone()
}

/// Register the process-server itself in the system
fn register_self() -> Result<(), kobject::Error> {
    let process = kobject::Process::current().clone();
    let main_thread = kobject::Thread::open_self()?;
    let pid = Pid(process.pid());

    let info = ProcessInfo {
        process,
        main_thread,
        environment: get_empty_kvblock(),
        arguments: get_empty_kvblock(),
        exit_code: None,
    };

    let mut processes = PROCESSES.write();
    processes.insert(pid, info);

    Ok(())
}

fn register_init() -> Result<(), kobject::Error> {
    const INIT_PID: u64 = 1;
    // Note: this is fishy, we should really find the main thread differently
    const INIT_MAIN_THREAD_TID: u64 = 3;

    let process = kobject::Process::open(INIT_PID)?;
    let main_thread = kobject::Thread::open(INIT_MAIN_THREAD_TID)?;
    let pid = Pid(process.pid());

    let info = ProcessInfo {
        process,
        main_thread,
        environment: get_empty_kvblock(),
        arguments: get_empty_kvblock(),
        exit_code: None,
    };

    let mut processes = PROCESSES.write();
    processes.insert(pid, info);

    Ok(())
}

#[no_mangle]
pub fn main() {
    let server = ipc::ServerBuilder::new(messages::PORT_NAME, messages::VERSION)
        .with_handler(messages::Type::CreateProcess, create_process)
        .build()
        .expect("failed to build process-server IPC server");

    info!("process-server started");

    server.run();
}

fn create_process(
    query: messages::CreateProcessQueryParameters,
    mut query_handles: ipc::Handles,
) -> Result<(messages::CreateProcessReply, ipc::Handles), messages::ProcessServerError> {
    let name_handle = query_handles.take(messages::CreateProcessQueryParameters::HANDLE_NAME_MOBJ);

    let name_reader =
        BufferReader::new(name_handle, &query.name).expect("failed to create buffer reader");
    let str = unsafe { str::from_utf8_unchecked(name_reader.buffer()) };

    info!("Creating process {}", str);

    Err(messages::ProcessServerError::InvalidArgument)
}
