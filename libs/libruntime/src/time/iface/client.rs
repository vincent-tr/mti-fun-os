use super::messages;
use crate::{ipc, time::DateTime};

pub type TimeServerCallError = ipc::CallError<messages::TimeServerError>;

/// Low level time client implementation.
#[derive(Debug)]
pub struct Client {
    ipc_client: ipc::Client<'static>,
}

impl Client {
    /// Creates a new time client.
    pub fn new() -> Self {
        Self {
            ipc_client: ipc::Client::new(messages::PORT_NAME, messages::VERSION),
        }
    }

    /// call ipc GetWallTime
    pub fn get_wall_time(&self) -> Result<DateTime, TimeServerCallError> {
        let query = messages::GetWallTimeQueryParameters {};
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::GetWallTimeQueryParameters, messages::GetWallTimeReply, messages::TimeServerError>(
            messages::Type::GetWallTime,
            query,
            query_handles,
        )?;

        let wall_time = DateTime::try_from(reply.timestamp).expect("Could not load timestamp");

        Ok(wall_time)
    }
}
