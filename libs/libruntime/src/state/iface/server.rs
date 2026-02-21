use core::fmt;

use alloc::{string::String, sync::Arc};
use log::error;

use crate::{
    ipc,
    kobject::{self, KObject},
};

use super::{StateServerError, messages};

pub trait StateServer {
    type Error: Into<StateServerError>;

    fn process_terminated(&self, _pid: u64) {}

    fn get_state(&self, sender_id: u64, name: &str) -> Result<kobject::MemoryObject, Self::Error>;
}

/// The main server structure
#[derive(Debug)]
pub struct Server<Impl: StateServer + 'static> {
    inner: Impl,
}

impl<Impl: StateServer + 'static> Server<Impl> {
    pub fn new(inner: Impl) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::Server, kobject::Error> {
        let builder = ipc::ManagedServerBuilder::<_, StateServerError, StateServerError>::new(
            &self,
            messages::PORT_NAME,
            messages::VERSION,
        );
        let builder = builder.with_process_exit_handler(Self::process_terminated_handler);

        let builder = builder.with_handler(messages::Type::GetState, Self::get_state_handler);

        builder.build()
    }

    fn process_terminated_handler(&self, pid: u64) {
        self.inner.process_terminated(pid);
    }

    fn get_state_handler(
        &self,
        query: messages::GetStateQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::GetStateReply, ipc::KHandles), StateServerError> {
        let name_view = {
            let handle = query_handles.take(messages::GetStateQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let name = String::from(unsafe { name_view.str() });

        let mobj = self.inner.get_state(sender_id, &name).map_err(Into::into)?;

        let mut reply_handles = ipc::KHandles::new();
        reply_handles[messages::GetStateReply::HANDLE_VALUE_MOBJ] = mobj.into_handle();

        Ok((messages::GetStateReply {}, reply_handles))
    }
}

/// Extension trait for Result to add context
trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, StateServerError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, StateServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            StateServerError::InvalidArgument
        })
    }
}
