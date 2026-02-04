mod client;
mod kvblock;
pub mod messages;

use alloc::{string::String, vec::Vec};
pub use kvblock::KVBlock;

use crate::{ipc, sync::RwLock};

lazy_static::lazy_static! {
    static ref CLIENT: client::Client = client::Client::new();
}

type ProcessServerError = ipc::CallError<messages::ProcessServerError>;

#[derive(Debug)]
pub struct Process {
    handle: ipc::Handle,
}

impl Process {
    pub fn spawn(
        name: &str,
        binary: ipc::Buffer<'_>,
        env: &[(&str, &str)],
        args: &[(&str, &str)],
    ) -> Result<Self, ProcessServerError> {
        let handle = CLIENT.create_process(name, binary, env, args)?;

        Ok(Self { handle })
    }
}

/// Represents the current process.
#[derive(Debug)]
pub struct SelfProcess {
    name: RwLock<String>,
    env: RwLock<KVBlock>,
    args: KVBlock,
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
        let mut entries = Vec::with_capacity(env.len());

        for (entry_key, entry_value) in env.iter() {
            entries.push((String::from(entry_key), String::from(entry_value)));
        }

        entries
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

    /// Set the exit code of the process
    pub fn set_exit_code(&self, code: i32) {
        CLIENT.set_exit_code(code).expect("failed to set exit code");
    }
}
