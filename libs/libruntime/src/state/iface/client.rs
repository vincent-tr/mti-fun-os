use super::messages;
use crate::{
    ipc,
    kobject::{self, KObject},
};

pub type StateServerCallError = ipc::CallError<messages::StateServerError>;

/// Low level process client implementation.
#[derive(Debug)]
pub struct Client {
    ipc_client: ipc::Client<'static>,
}

impl Client {
    /// Creates a new process client.
    pub fn new() -> Self {
        Self {
            ipc_client: ipc::Client::new(messages::PORT_NAME, messages::VERSION),
        }
    }

    /// call ipc GetState
    pub fn get_state(&self, name: &str) -> Result<kobject::MemoryObject, StateServerCallError> {
        let (name_memobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::GetStateQueryParameters { name: name_buffer };
        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::GetStateQueryParameters::HANDLE_NAME_MOBJ] =
            name_memobj.into_handle();

        let (_reply, mut reply_handles) = self.ipc_client.call::<messages::Type, messages::GetStateQueryParameters, messages::GetStateReply, messages::StateServerError>(
            messages::Type::GetState,
            query,
            query_handles,
        )?;

        let mobj = kobject::MemoryObject::from_handle(
            reply_handles.take(messages::GetStateReply::HANDLE_VALUE_MOBJ),
        )
        .expect("could not get value memory object");

        Ok(mobj)
    }
}
