use crate::{ipc, kobject};
use alloc::sync::Arc;

use super::{TimeServerError, messages};

/// Time server interface
pub trait TimeServer {
    type Error: Into<TimeServerError>;

    fn get_wall_time(&self, sender_id: u64) -> Result<::time::UtcDateTime, Self::Error>;
}

/// The main server structure
#[derive(Debug)]
pub struct Server<Impl: TimeServer + 'static> {
    inner: Impl,
}

impl<Impl: TimeServer + 'static> Server<Impl> {
    pub fn new(inner: Impl) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::Server, kobject::Error> {
        let builder = ipc::ManagedServerBuilder::<_, TimeServerError, TimeServerError>::new(
            &self,
            messages::PORT_NAME,
            messages::VERSION,
        );

        let builder =
            builder.with_handler(messages::Type::GetWallTime, Self::get_wall_time_handler);

        builder.build()
    }

    fn get_wall_time_handler(
        &self,
        _query: messages::GetWallTimeQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetWallTimeReply, ipc::KHandles), TimeServerError> {
        let timestamp = self.inner.get_wall_time(sender_id).map_err(Into::into)?;

        Ok((
            messages::GetWallTimeReply {
                timestamp: timestamp.unix_timestamp_nanos(),
            },
            ipc::KHandles::new(),
        ))
    }
}
