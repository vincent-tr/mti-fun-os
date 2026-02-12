use core::future::Future;

use crate::error::{InternalError, ResultExt};
use alloc::{string::String, sync::Arc, vec::Vec};
use libruntime::{
    ipc,
    kobject::{self, KObject},
    vfs::messages,
};
use log::{debug, info, warn};

/// The main manager structure
#[derive(Debug)]
pub struct Manager {}

impl Manager {
    pub fn new() -> Result<Arc<Self>, kobject::Error> {
        let manager = Self {};

        Ok(Arc::new(manager))
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::AsyncServer, kobject::Error> {
        let builder =
            ipc::ManagedAsyncServerBuilder::<_, InternalError, messages::VfsServerError>::new(
                self,
                messages::PORT_NAME,
                messages::VERSION,
            );

        let builder = builder.with_process_exit_handler(Self::process_terminated);

        let builder = builder.with_handler(messages::Type::Close, Self::close_handler);

        builder.build()
    }

    async fn process_terminated(self: Arc<Self>, pid: u64) {}

    async fn close_handler(
        self: Arc<Self>,
        _query: messages::CloseQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CloseReply, ipc::KHandles), InternalError> {
        libruntime::timer::async_sleep(libruntime::timer::Duration::from_seconds(1)).await;

        let reply = messages::CloseReply {};
        let reply_handles = ipc::KHandles::new();

        Ok((reply, reply_handles))
    }
}
