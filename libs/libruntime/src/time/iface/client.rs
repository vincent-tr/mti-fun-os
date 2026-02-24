use core::ptr;

use super::messages;
use crate::ipc;

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
    pub fn get_wall_time(&self) -> Result<::time::UtcDateTime, TimeServerCallError> {
        let query = messages::GetWallTimeQueryParameters {};
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::GetWallTimeQueryParameters, messages::GetWallTimeReply, messages::TimeServerError>(
            messages::Type::GetWallTime,
            query,
            query_handles,
        )?;

        let timestamp = unsafe { ptr::read_unaligned(reply.timestamp.as_ptr() as *const i128) };

        let wall_time = ::time::UtcDateTime::from_unix_timestamp_nanos(timestamp)
            .expect("Could not load timestamp");

        Ok(wall_time)
    }
}
