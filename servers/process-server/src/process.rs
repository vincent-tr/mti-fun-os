use core::fmt;

use alloc::{string::String, sync::Arc};
use hashbrown::HashMap;
use libruntime::{collections::WeakMap, kobject, process::KVBlock, sync::RwLock};

/// Process ID
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct Pid(u64);

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

/// Process information stored in the server
#[derive(Debug)]
pub struct ProcessInfo {
    process: kobject::Process,
    main_thread: kobject::Thread,
    name: String,
    environment: KVBlock,
    arguments: KVBlock,
    exit_code: Option<i32>,
    exited: bool,
}

impl ProcessInfo {
    pub fn new(
        process: kobject::Process,
        main_thread: kobject::Thread,
        name: String,
        environment: KVBlock,
        arguments: KVBlock,
    ) -> Arc<Self> {
        let info = Arc::new(Self {
            process,
            main_thread,
            name,
            environment,
            arguments,
            exit_code: None,
            exited: false,
        });

        PROCESSES.insert(&info);

        info
    }

    pub fn pid(&self) -> Pid {
        Pid::from(self.process.pid())
    }
}

impl Drop for ProcessInfo {
    fn drop(&mut self) {
        PROCESSES.remove(self.pid());
    }
}

lazy_static! {
    pub static ref LIVE_PROCESSES: LiveProcesses = LiveProcesses::new();
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
    pub fn insert(&mut self, info: Arc<ProcessInfo>) {
        let mut processes = self.processes.write();

        let pid = info.pid();
        processes.insert(pid, info);
    }

    /// Remove a process from the list by its PID
    pub fn remove(&mut self, pid: Pid) {
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
    pub static ref PROCESSES: Processes = Processes::new();
}

/// List of all processes, both live and exited (if opened), for information purposes
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
    fn insert(&mut self, info: &Arc<ProcessInfo>) {
        let mut processes = self.processes.write();

        let pid = info.pid();
        processes.insert(pid, info);
    }

    /// Remove a process from the list by its PID
    ///
    /// Reserved for ProcessInfo drop
    fn remove(&mut self, pid: Pid) {
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
