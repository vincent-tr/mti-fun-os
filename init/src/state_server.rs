use core::fmt;
use log::error;

use hashbrown::HashMap;

use alloc::string::String;
use libruntime::{
    kobject,
    state::iface::{STATE_SIZE, StateServer, StateServerError, build_ipc_server},
    sync::RwLock,
};
use log::info;

pub fn start() {
    let server = Server::new();
    let ipc_server = build_ipc_server(server).expect("failed to build state-server IPC server");

    let mut options = kobject::ThreadOptions::default();
    options.name("state-server");

    let entry = move || {
        ipc_server.run();
    };

    kobject::Thread::start(entry, options).expect("failed to start state-server thread");
}

/// The main server structure
#[derive(Debug)]
struct Server {
    state: RwLock<HashMap<String, kobject::MemoryObject>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }
}

impl StateServer for Server {
    type Error = StateServerError;

    fn get_state(&self, sender_id: u64, name: &str) -> Result<kobject::MemoryObject, Self::Error> {
        let mut state = self.state.write();

        if name.is_empty() {
            error!(
                "Received GetState request with empty name from sender {}",
                sender_id
            );
            return Err(StateServerError::InvalidArgument);
        }

        let name = String::from(name);

        let mobj = if let Some(mobj) = state.get(&name) {
            mobj.clone()
        } else {
            info!("Creating new state for '{}'", name);
            let mobj = kobject::MemoryObject::create(STATE_SIZE)
                .runtime_err("Failed to create memory object")?;
            state.insert(name, mobj.clone());
            mobj
        };

        Ok(mobj)
    }
}

/// Extension trait for Result to add context
trait ResultExt<T> {
    fn runtime_err(self, msg: &'static str) -> Result<T, StateServerError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn runtime_err(self, msg: &'static str) -> Result<T, StateServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            StateServerError::RuntimeError
        })
    }
}
