mod client;
mod kvblock;
pub mod messages;

use alloc::string::String;
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
