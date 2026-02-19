pub mod iface;

use alloc::{string::String, vec::Vec};
use log::debug;

use crate::{ipc, kobject, sync::RwLock};

use iface::{Client, KVBlock, ProcessInfo, ProcessStatus, ProcessTerminatedNotification, SymBlock};

lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::new();
}

pub type ProcessServerError = ipc::CallError<iface::ProcessServerError>;

/// High level process management API.
#[derive(Debug)]
pub struct Process {
    handle: ipc::Handle,
    pid: u64,
}

impl Process {
    /// Spawn a new process with the given name, binary, environment variables and arguments.
    pub fn spawn(
        name: &str,
        binary: ipc::Buffer<'_>,
        env: &[(&str, &str)],
        args: &[(&str, &str)],
    ) -> Result<Self, ProcessServerError> {
        let (handle, pid) = CLIENT.create_process(name, binary, env, args)?;

        Ok(Self { handle, pid })
    }

    /// Open an existing process by its PID.
    pub fn open(pid: u64) -> Result<Self, ProcessServerError> {
        let (handle, pid) = CLIENT.open_process(pid)?;

        Ok(Self { handle, pid })
    }

    /// Get the PID of the process.
    pub fn pid(&self) -> u64 {
        self.pid
    }

    /// Get the name of the process.
    pub fn name(&self) -> String {
        CLIENT
            .get_process_name(self.handle)
            .expect("failed to get process name")
    }

    /// Get the environment variables of the process.
    pub fn env(&self) -> Vec<(String, String)> {
        let env = CLIENT
            .get_process_env(self.handle)
            .expect("failed to get process environment");

        block_to_vec(&env)
    }

    /// Get the arguments of the process.
    pub fn args(&self) -> Vec<(String, String)> {
        let args = CLIENT
            .get_process_args(self.handle)
            .expect("failed to get process arguments");

        block_to_vec(&args)
    }

    /// Get the status of the process.
    pub fn status(&self) -> ProcessStatus {
        CLIENT
            .get_process_status(self.handle)
            .expect("failed to get process status")
    }

    /// Kill the process
    pub fn kill(&self) -> Result<(), ProcessServerError> {
        CLIENT.terminate_process(self.handle)
    }

    /// Create a waiter for this process, allowing to wait for its termination.
    pub fn create_waiter(&self) -> Result<ProcessWaiter, ProcessServerError> {
        ProcessWaiter::new(self)
    }

    /// List all processes in the system.
    pub fn list() -> Result<Vec<ProcessInfo>, ProcessServerError> {
        CLIENT.list_processes()
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        CLIENT
            .close_process(self.handle)
            .expect("failed to close process");
    }
}

/// Represents a waiter for a process, allowing to wait for its termination.
///
/// This structure is not thread safe, and should only be used by the thread that created it.
#[derive(Debug)]
pub struct ProcessWaiter {
    reader: kobject::PortReceiver,
    registration_handle: ipc::Handle,
    status: ProcessStatus,
}

impl kobject::KWaitable for ProcessWaiter {
    unsafe fn waitable_handle(&self) -> &libsyscalls::Handle {
        assert!(self.status == ProcessStatus::Running);

        self.reader.waitable_handle()
    }

    fn wait(&self) -> Result<(), kobject::Error> {
        assert!(self.status == ProcessStatus::Running);

        self.reader.wait()
    }
}

impl ProcessWaiter {
    /// Create a new ProcessWaiter for the given process.
    pub fn new(process: &Process) -> Result<Self, ProcessServerError> {
        let (reader, sender) = kobject::Port::create(None).expect("failed to create port");

        let correlation = 0;
        let registration_handle =
            CLIENT.register_process_terminated_notification(process.handle, correlation, sender)?;

        Ok(Self {
            reader,
            registration_handle,
            status: ProcessStatus::Running,
        })
    }

    /// Check if this object is waitable.
    ///
    /// Calling KWaitable API on it when it's not waitable will cause a panic.
    pub fn is_waitable(&self) -> bool {
        self.status == ProcessStatus::Running
    }

    /// Get the current status of the process.
    ///
    /// Note that this will also update the status if the process has already terminated.
    pub fn status(&mut self) -> ProcessStatus {
        let res = self.reader.receive();

        if let Err(kobject::Error::ObjectNotReady) = res {
            // Still running
            return self.status.clone();
        }

        let msg = res.expect("failed to receive process status message");

        self.process_message(msg);
        self.status.clone()
    }

    /// Wait for the process to terminate and return its exit code.
    pub fn wait_status(&mut self) -> i32 {
        if let ProcessStatus::Exited(exit_code) = self.status {
            return exit_code;
        };

        let msg = self
            .reader
            .blocking_receive()
            .expect("failed to receive process status message");

        self.process_message(msg);

        if let ProcessStatus::Exited(exit_code) = self.status {
            exit_code
        } else {
            panic!("unexpected process status");
        }
    }

    fn process_message(&mut self, msg: kobject::Message) {
        let notification = unsafe { msg.data::<ProcessTerminatedNotification>() };
        let exit_code = notification.exit_code;
        debug!("process terminated with exit code {}", exit_code);

        self.status = ProcessStatus::Exited(exit_code);

        // The handle is invalid after receiving the notification, so we set it to INVALID to avoid trying to unregister it in the Drop implementation.
        self.registration_handle = ipc::Handle::INVALID;
    }
}

impl Drop for ProcessWaiter {
    fn drop(&mut self) {
        if self.registration_handle == ipc::Handle::INVALID {
            return;
        }

        if let Err(e) = CLIENT.unregister_process_terminated_notification(self.registration_handle)
        {
            // Note: do not panic, since if the process died before we read it, unregister will fail.
            log::error!("failed to deregister process waiter: {:?}", e);
        }
    }
}

/// Represents the current process.
#[derive(Debug)]
pub struct SelfProcess {
    name: RwLock<String>,
    env: RwLock<KVBlock>,
    args: KVBlock,
    symbols: SymBlock,
}

impl SelfProcess {
    /// Get the current process
    pub fn get() -> &'static Self {
        lazy_static::lazy_static! {
          static ref CURRENT: SelfProcess = SelfProcess::new();
        }

        &CURRENT
    }

    /// Create a new SelfProcess instance, with data fetched from the process server.
    fn new() -> Self {
        let startup_info = CLIENT
            .get_startup_info()
            .expect("failed to get startup info");

        Self {
            name: RwLock::new(startup_info.name),
            env: RwLock::new(startup_info.env),
            args: startup_info.args,
            symbols: startup_info.symbols,
        }
    }

    /// Get the process name
    pub fn name(&self) -> String {
        self.name.read().clone()
    }

    /// Set the process name
    pub fn set_name(&self, name: &str) {
        let mut name_lock = self.name.write();

        *name_lock = String::from(name);
        CLIENT.update_name(name).expect("failed to update name");
    }

    /// Get the process environment variable
    pub fn env(&self, key: &str) -> Option<String> {
        let env = self.env.read();

        for (entry_key, entry_value) in env.iter() {
            if entry_key == key {
                return Some(String::from(entry_value));
            }
        }

        None
    }

    /// Set the process environment variable
    pub fn set_env(&self, key: &str, value: &str) {
        let mut env = self.env.write();
        let mut found = false;

        // Create a new KVBlock with the updated environment variable
        let mut new_entries = Vec::new();
        for (entry_key, entry_value) in env.iter() {
            if entry_key == key {
                new_entries.push((key, value));
                found = true;
            } else {
                new_entries.push((entry_key, entry_value));
            }
        }

        if !found {
            new_entries.push((key, value));
        }

        let mobj = KVBlock::build(&new_entries);

        *env = KVBlock::from_memory_object(mobj.clone()).expect("failed to build KVBlock");
        CLIENT
            .update_env(mobj)
            .expect("failed to update environment");
    }

    /// Get all environment variables
    pub fn env_all(&self) -> Vec<(String, String)> {
        let env = self.env.read();
        block_to_vec(&env)
    }

    /// Replace all environment variables
    pub fn replace_env(&self, new_env: &[(&str, &str)]) {
        let mobj = KVBlock::build(new_env);

        let mut env = self.env.write();
        *env = KVBlock::from_memory_object(mobj.clone()).expect("failed to build KVBlock");
        CLIENT
            .update_env(mobj)
            .expect("failed to update environment");
    }

    /// Get the process argument
    pub fn arg(&self, key: &str) -> Option<String> {
        for (entry_key, entry_value) in self.args.iter() {
            if entry_key == key {
                return Some(String::from(entry_value));
            }
        }

        None
    }

    /// Get all arguments
    pub fn args_all(&self) -> Vec<(String, String)> {
        let mut entries = Vec::with_capacity(self.args.len());

        for (entry_key, entry_value) in self.args.iter() {
            entries.push((String::from(entry_key), String::from(entry_value)));
        }

        entries
    }

    /// Get the process symbols
    pub fn symbols(&self) -> &SymBlock {
        &self.symbols
    }

    /// Set the exit code of the process
    pub fn set_exit_code(&self, code: i32) {
        CLIENT.set_exit_code(code).expect("failed to set exit code");
    }
}

fn block_to_vec(block: &KVBlock) -> Vec<(String, String)> {
    let mut entries = Vec::with_capacity(block.len());

    for (entry_key, entry_value) in block.iter() {
        entries.push((String::from(entry_key), String::from(entry_value)));
    }

    entries
}
