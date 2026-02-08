use core::{
    fmt,
    sync::atomic::{AtomicBool, AtomicI32, Ordering},
};

use alloc::{string::String, sync::Arc, vec::Vec};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use libruntime::{
    collections::WeakMap,
    ipc, kobject,
    process::{messages, KVBlock},
    sync::RwLock,
};
use log::info;

/// Process ID
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct Pid(u64);

impl Pid {
    /// Get the raw u64 value of this PID, which is used in IPC and kernel objects
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Invalid PID, used to represent errors or non-existent processes
    pub const INVALID: Self = Self(0);
}

impl From<u64> for Pid {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Exit code of a process
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct ExitCode(i32);

impl ExitCode {
    /// Exit code used when a process exits successfully
    pub const SUCCESS: Self = Self(messages::EXIT_CODE_SUCCESS);

    /// Exit code used when a process has not exited yet, or the exit code has not been reported by the process
    pub const UNSET: Self = Self(messages::EXIT_CODE_UNSET);

    /// Exit code used when a process is killed
    pub const KILLED: Self = Self(messages::EXIT_CODE_KILLED);

    /// Minimum value for user-defined exit codes. Values below this are reserved for special meanings (like UNSET and KILLED).
    const RESERVED_MIN: i32 = i32::MIN + 10;

    /// Get the raw i32 value of this exit code, which is used in IPC
    pub fn as_i32(&self) -> i32 {
        self.0
    }
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::SUCCESS => write!(f, "SUCCESS"),
            Self::UNSET => write!(f, "UNSET"),
            Self::KILLED => write!(f, "KILLED"),
            code => write!(f, "{}", code.0),
        }
    }
}

impl TryFrom<i32> for ExitCode {
    type Error = ExitCodeConvertError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value >= Self::RESERVED_MIN {
            Ok(Self(value))
        } else {
            Err(ExitCodeConvertError)
        }
    }
}

pub struct ExitCodeConvertError;

impl fmt::Display for ExitCodeConvertError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Exit code must be greater than or equal to {}",
            ExitCode::RESERVED_MIN
        )
    }
}

/// Process information stored in the server
#[derive(Debug)]
pub struct ProcessInfo {
    process: kobject::Process,
    main_thread: kobject::Thread,
    creator: Pid,
    name: RwLock<String>,
    environment: RwLock<KVBlock>,
    arguments: KVBlock,
    exit_code: AtomicI32,
    terminated: AtomicBool,
}

impl ProcessInfo {
    pub fn new(
        creator: Pid,
        process: kobject::Process,
        main_thread: kobject::Thread,
        name: String,
        environment: KVBlock,
        arguments: KVBlock,
    ) -> Arc<Self> {
        let info = Arc::new(Self {
            process,
            main_thread,
            creator,
            name: RwLock::new(name),
            environment: RwLock::new(environment),
            arguments,
            exit_code: AtomicI32::new(ExitCode::UNSET.0),
            terminated: AtomicBool::new(false),
        });

        PROCESSES.insert(&info);
        LIVE_PROCESSES.insert(info.clone());

        info!("Created process {}: {}", info.name(), info.pid());

        info
    }

    /// Get the PID of this process
    pub fn pid(&self) -> Pid {
        Pid::from(self.process.pid())
    }

    /// Get the PID of the creator of this process (PPID)
    pub fn creator(&self) -> Pid {
        self.creator
    }

    /// Get the name of this process
    pub fn name(&self) -> String {
        self.name.read().clone()
    }

    /// Get the environment of this process
    pub fn environment(&self) -> KVBlock {
        let mobj = self.environment.read().memory_object().clone();
        KVBlock::from_memory_object(mobj).expect("failed to clone environment")
    }

    /// Get the arguments of this process
    pub fn arguments(&self) -> &KVBlock {
        &self.arguments
    }

    /// Check if this process is terminated
    pub fn is_terminated(&self) -> bool {
        self.terminated.load(Ordering::SeqCst)
    }

    /// Get the exit code of this process (UNSET if not terminated yet)
    pub fn exit_code(&self) -> ExitCode {
        ExitCode(self.exit_code.load(Ordering::SeqCst))
    }

    /// Set the exit code of this process
    pub fn set_exit_code(&self, code: ExitCode) {
        self.exit_code.store(code.0, Ordering::SeqCst);

        info!("Set process {} exit code to {}", self.pid(), code.0);
    }

    /// Update the name of this process
    pub fn update_name(&self, name: String) {
        let mut self_name = self.name.write();
        *self_name = name;

        info!("Updating process {} name to {}", self.pid(), *self_name);
    }

    /// Update the environment of this process
    pub fn update_environment(&self, environment: KVBlock) {
        let mut self_env = self.environment.write();
        *self_env = environment;

        info!("Updating process {} environment", self.pid());
    }

    /// Mark this process as terminated, making it unavailable in the live processes list
    pub fn mark_terminated(&self) {
        self.terminated.store(true, Ordering::SeqCst);

        LIVE_PROCESSES.remove(self.pid());

        info!("Process {} is terminated", self.pid());
    }

    /// Get the kernel object representing this process
    pub fn kobject_process(&self) -> &kobject::Process {
        &self.process
    }
}

impl Drop for ProcessInfo {
    fn drop(&mut self) {
        PROCESSES.remove(self.pid());
    }
}

/// Find a live process by its PID
pub fn find_live_process(pid: Pid) -> Option<Arc<ProcessInfo>> {
    LIVE_PROCESSES.get(pid)
}

/// Find a process by its PID, even if it has terminated (if still available)
pub fn find_process(pid: Pid) -> Option<Arc<ProcessInfo>> {
    PROCESSES.find(pid)
}

/// List all processes in the system, both live and terminated (if still available)
pub fn list_processes() -> Vec<Arc<ProcessInfo>> {
    PROCESSES.list()
}

lazy_static! {
    static ref LIVE_PROCESSES: LiveProcesses = LiveProcesses::new();
}

/// List of live processes
///
/// They are kept in this list to ensure they are not dropped while still running, and to be able to query their information
#[derive(Debug)]
pub struct LiveProcesses {
    processes: RwLock<HashMap<Pid, Arc<ProcessInfo>>>,
}

impl LiveProcesses {
    /// Create a new empty list of live processes
    fn new() -> Self {
        Self {
            processes: RwLock::new(HashMap::new()),
        }
    }

    /// Insert a new process into the list
    pub fn insert(&self, info: Arc<ProcessInfo>) {
        let mut processes = self.processes.write();

        let pid = info.pid();
        processes.insert(pid, info);
    }

    /// Remove a process from the list by its PID
    pub fn remove(&self, pid: Pid) {
        let mut processes = self.processes.write();

        processes.remove(&pid);
    }

    /// Get a process information by its PID
    pub fn get(&self, pid: Pid) -> Option<Arc<ProcessInfo>> {
        let processes = self.processes.read();

        processes.get(&pid).cloned()
    }
}

lazy_static! {
    static ref PROCESSES: Processes = Processes::new();
}

/// List of all processes, both live and terminated (if opened), for information purposes
#[derive(Debug)]
pub struct Processes {
    processes: RwLock<WeakMap<Pid, ProcessInfo>>,
}

impl Processes {
    /// Create a new empty list of processes
    fn new() -> Self {
        Self {
            processes: RwLock::new(WeakMap::new()),
        }
    }

    /// Insert a new process into the list
    ///
    /// Reserved for ProcessInfo creation
    fn insert(&self, info: &Arc<ProcessInfo>) {
        let mut processes = self.processes.write();

        let pid = info.pid();
        processes.insert(pid, info);
    }

    /// Remove a process from the list by its PID
    ///
    /// Reserved for ProcessInfo drop
    fn remove(&self, pid: Pid) {
        let mut processes = self.processes.write();

        processes.remove(pid);
    }

    /// Get a process information by its PID
    pub fn find(&self, pid: Pid) -> Option<Arc<ProcessInfo>> {
        let processes = self.processes.read();

        processes.find(&pid)
    }

    /// List all processes in the system
    pub fn list(&self) -> Vec<Arc<ProcessInfo>> {
        let processes = self.processes.read();

        let mut values = Vec::new();

        for pid in processes.keys() {
            if let Some(info) = processes.find(&pid) {
                values.push(info);
            }
        }

        values
    }
}
