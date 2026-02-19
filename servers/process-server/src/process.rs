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
    process::iface::{
        KVBlock, ProcessTerminatedNotification, SymBlock, EXIT_CODE_KILLED, EXIT_CODE_SUCCESS,
        EXIT_CODE_UNSET,
    },
    sync::RwLock,
};
use log::{debug, error, info};

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
    pub const SUCCESS: Self = Self(EXIT_CODE_SUCCESS);

    /// Exit code used when a process has not exited yet, or the exit code has not been reported by the process
    pub const UNSET: Self = Self(EXIT_CODE_UNSET);

    /// Exit code used when a process is killed
    pub const KILLED: Self = Self(EXIT_CODE_KILLED);

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
pub struct Process {
    process: kobject::Process,
    _main_thread: kobject::Thread,
    creator: Pid,
    name: RwLock<String>,
    environment: RwLock<KVBlock>,
    arguments: KVBlock,
    symbols: SymBlock,
    exit_code: AtomicI32,
    terminated: AtomicBool,
    termination_registrations: RwLock<HashMap<ipc::Handle, TerminationRegistration>>,
}

impl Process {
    pub fn new(
        creator: Pid,
        process: kobject::Process,
        main_thread: kobject::Thread,
        name: String,
        environment: KVBlock,
        arguments: KVBlock,
        symbols: SymBlock,
    ) -> Arc<Self> {
        let info = Arc::new(Self {
            process,
            _main_thread: main_thread,
            creator,
            name: RwLock::new(name),
            environment: RwLock::new(environment),
            arguments,
            symbols,
            exit_code: AtomicI32::new(ExitCode::UNSET.0),
            terminated: AtomicBool::new(false),
            termination_registrations: RwLock::new(HashMap::new()),
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

    /// Get the symbol information of this process
    pub fn symbols(&self) -> &SymBlock {
        &self.symbols
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
    ///
    /// Also fire all registered termination notifications for this process, returning the fired notifications for information purposes
    pub fn mark_terminated(&self) {
        self.terminated.store(true, Ordering::SeqCst);

        LIVE_PROCESSES.remove(self.pid());

        info!("Process {} is terminated", self.pid());

        // Fire all notifications registered for this process, and remove them
        for (_, registration) in self.termination_registrations.write().drain() {
            registration.fire();

            TERMINATION_REGISTRATIONS.write().remove(&registration);
        }
    }

    /// Add a new process termination notification registration for this process
    pub fn add_termination_registration(&self, registration: TerminationRegistration) {
        if self.is_terminated() {
            // If the process is already terminated, fire the notification immediately
            registration.fire();
            return;
        }

        // Otherwise, add it to the list of registrations for this process
        TERMINATION_REGISTRATIONS.write().add(&registration);

        self.termination_registrations
            .write()
            .insert(registration.owner_handle, registration);
    }

    /// Remove a process termination notification registration for this process by the owner handle
    pub fn remove_termination_registration(&self, owner_handle: ipc::Handle) {
        let registration = self
            .termination_registrations
            .write()
            .remove(&owner_handle)
            .expect("faild to remove registration");

        TERMINATION_REGISTRATIONS.write().remove(&registration);
    }

    /// Get the owner PID of a process termination notification registration by the owner handle
    pub fn get_registration_owner(&self, owner_handle: ipc::Handle) -> Pid {
        self.termination_registrations
            .read()
            .get(&owner_handle)
            .expect("data inconsistency: registration not found")
            .owner()
    }

    /// Get the kernel object representing this process
    pub fn kobject_process(&self) -> &kobject::Process {
        &self.process
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        PROCESSES.remove(self.pid());
    }
}

/// Find a live process by its PID
pub fn find_live_process(pid: Pid) -> Option<Arc<Process>> {
    LIVE_PROCESSES.get(pid)
}

/// Find a process by its PID, even if it has terminated (if still available)
pub fn find_process(pid: Pid) -> Option<Arc<Process>> {
    PROCESSES.find(pid)
}

/// List all processes in the system, both live and terminated (if still available)
pub fn list_processes() -> Vec<Arc<Process>> {
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
    processes: RwLock<HashMap<Pid, Arc<Process>>>,
}

impl LiveProcesses {
    /// Create a new empty list of live processes
    fn new() -> Self {
        Self {
            processes: RwLock::new(HashMap::new()),
        }
    }

    /// Insert a new process into the list
    pub fn insert(&self, info: Arc<Process>) {
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
    pub fn get(&self, pid: Pid) -> Option<Arc<Process>> {
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
    processes: RwLock<WeakMap<Pid, Process>>,
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
    /// Reserved for Process creation
    fn insert(&self, info: &Arc<Process>) {
        let mut processes = self.processes.write();

        let pid = info.pid();
        processes.insert(pid, info);
    }

    /// Remove a process from the list by its PID
    ///
    /// Reserved for Process drop
    fn remove(&self, pid: Pid) {
        let mut processes = self.processes.write();

        processes.remove(pid);
    }

    /// Get a process information by its PID
    pub fn find(&self, pid: Pid) -> Option<Arc<Process>> {
        let processes = self.processes.read();

        processes.find(&pid)
    }

    /// List all processes in the system
    pub fn list(&self) -> Vec<Arc<Process>> {
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

/// Registration for process termination notifications
#[derive(Debug)]
pub struct TerminationRegistration {
    owner: Pid,
    owner_handle: ipc::Handle,
    process: Arc<Process>,
    port: kobject::PortSender,
    correlation: u64,
}

impl TerminationRegistration {
    /// Create a new notification registration
    pub fn new(
        owner: Pid,
        owner_handle: ipc::Handle,
        process: Arc<Process>,
        port: kobject::PortSender,
        correlation: u64,
    ) -> Self {
        Self {
            owner,
            owner_handle,
            process,
            port,
            correlation,
        }
    }

    /// Fire the notification, sending a message to the registered port with the process termination information
    pub fn fire(&self) {
        let notification = ProcessTerminatedNotification {
            correlation: self.correlation,
            pid: self.process.pid().as_u64(),
            exit_code: self.process.exit_code().as_i32(),
        };

        let handles = ipc::KHandles::new();
        let mut msg = unsafe { kobject::Message::new(&notification, handles.into()) };

        if let Err(e) = self.port.send(&mut msg) {
            error!(
                "Failed to send process termination notification to {} for process {}: {}",
                self.owner,
                self.process.pid(),
                e
            );
        }

        debug!(
            "Fired process termination notification to {} for process {} with exit code {}",
            self.owner,
            self.process.pid(),
            self.process.exit_code()
        );
    }

    /// Get a pointer representing this registration, which can be used to identify it in the global TERMINATION_REGISTRATIONS list
    pub fn pointer(&self) -> RegistrationPointer {
        (self.process.pid(), self.owner_handle)
    }

    /// Get the owner PID of this registration
    pub fn owner(&self) -> Pid {
        self.owner
    }
}

pub type RegistrationPointer = (Pid, ipc::Handle);

/// Registrations for process terminations
#[derive(Debug)]
pub struct TerminationRegistrations {
    /// Map of all registration handles to their registration information, for quick access by handle
    by_handle: HashMap<ipc::Handle, Pid>,
    /// Map of owner PIDs to their registration handles and information, for quick access by owner and for bulk removals when an owner process terminates
    by_owner: HashMap<Pid, HashMap<ipc::Handle, Pid>>,
}

impl TerminationRegistrations {
    /// Create a new empty set of termination registrations
    pub fn new() -> Self {
        Self {
            by_handle: HashMap::new(),
            by_owner: HashMap::new(),
        }
    }

    /// Add a new registration
    ///
    /// Reserved for Process::add_termination_registration/remove_termination_registration/mark_terminated
    pub fn add(&mut self, registration: &TerminationRegistration) {
        let (target, handle) = registration.pointer();
        let owner = registration.owner();

        self.by_handle.insert(handle, target);

        self.by_owner
            .entry(owner)
            .or_insert_with(HashMap::new)
            .insert(handle, target);
    }

    /// Remove a registration by the owner handle
    ///
    /// Reserved for Process::add_termination_registration/remove_termination_registration/mark_terminated
    pub fn remove(&mut self, registration: &TerminationRegistration) {
        let (_, handle) = registration.pointer();
        let owner = registration.owner();

        self.by_handle
            .remove(&handle)
            .expect("could not remove registration from by_handle");

        let owner_registrations = self
            .by_owner
            .get_mut(&owner)
            .expect("could not find registration owner in by_owner");

        owner_registrations
            .remove(&handle)
            .expect("could not remove registration from by_owner");

        if owner_registrations.is_empty() {
            self.by_owner.remove(&owner);
        }
    }

    /// Get a registration by the owner handle
    pub fn get_by_handle(&self, owner_handle: ipc::Handle) -> Option<RegistrationPointer> {
        if let Some(target) = self.by_handle.get(&owner_handle) {
            Some((*target, owner_handle))
        } else {
            None
        }
    }

    /// Get all registrations for a given owner PID
    pub fn get_by_owner(&self, owner: Pid) -> Vec<RegistrationPointer> {
        let registrations = if let Some(registrations) = self.by_owner.get(&owner) {
            registrations
        } else {
            return Vec::new();
        };

        registrations
            .iter()
            .map(|(handle, target)| (*target, *handle))
            .collect()
    }
}

lazy_static! {
    static ref TERMINATION_REGISTRATIONS: RwLock<TerminationRegistrations> =
        RwLock::new(TerminationRegistrations::new());
}

/// List all process termination notification registrations for a given owner PID, used when an owner process terminates to fire all its registered notifications
pub fn list_termination_registrations_by_owner(owner: Pid) -> Vec<RegistrationPointer> {
    TERMINATION_REGISTRATIONS.read().get_by_owner(owner)
}

/// Unregister a process termination notification by the owner handle, used when an owner process explicitly wants to unregister a notification
pub fn get_termination_registration_by_handle(
    owner_handle: ipc::Handle,
) -> Option<RegistrationPointer> {
    TERMINATION_REGISTRATIONS.read().get_by_handle(owner_handle)
}
