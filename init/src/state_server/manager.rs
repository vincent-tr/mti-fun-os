use hashbrown::HashMap;

use super::error::{InternalError, ResultExt};
use alloc::{string::String, sync::Arc};
use libruntime::{
    ipc,
    kobject::{self, KObject},
    state::messages,
    sync::RwLock,
};
use log::info;

/// The main server structure
#[derive(Debug)]
pub struct Manager {
    state: RwLock<HashMap<String, kobject::MemoryObject>>,
}

impl Manager {
    pub fn new() -> Result<Arc<Self>, kobject::Error> {
        let manager = Manager {
            state: RwLock::new(HashMap::new()),
        };

        Ok(Arc::new(manager))
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::Server, kobject::Error> {
        let builder = ipc::ServerBuilder::new(messages::PORT_NAME, messages::VERSION);
        let builder = self.add_handler(builder, messages::Type::GetState, Self::get_state_handler);

        builder.build()
    }

    fn get_state_handler(
        &self,
        query: messages::GetStateQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::GetStateReply, ipc::KHandles), InternalError> {
        let mut state = self.state.write();

        let buffer_view = {
            let handle = query_handles.take(messages::GetStateQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let name = String::from(unsafe { buffer_view.str() });

        if name.is_empty() {
            return Err(InternalError::invalid_argument(
                "State name cannot be empty",
            ));
        }

        let mobj = if let Some(mobj) = state.get(&name) {
            mobj.clone()
        } else {
            info!("Creating new state for '{}'", name);
            let mobj = kobject::MemoryObject::create(messages::STATE_SIZE)
                .runtime_err("Failed to create memory object")?;
            state.insert(name, mobj.clone());
            mobj
        };

        let reply = messages::GetStateReply {};
        let mut reply_handles = ipc::KHandles::new();

        reply_handles[messages::GetStateReply::HANDLE_VALUE_MOBJ] = mobj.into_handle();

        Ok((reply, reply_handles))
    }

    fn add_handler<QueryParameters, ReplyContent>(
        self: &Arc<Self>,
        builder: ipc::ServerBuilder,
        message_type: messages::Type,
        handler: fn(
            &Self,
            QueryParameters,
            ipc::KHandles,
            u64,
        ) -> Result<(ReplyContent, ipc::KHandles), InternalError>,
    ) -> ipc::ServerBuilder
    where
        QueryParameters: Copy + 'static,
        ReplyContent: Copy + 'static,
    {
        let manager = Arc::clone(self);
        builder.with_handler(message_type, move |query, handles, sender_id| {
            handler(&manager, query, handles, sender_id).map_err(|e| e.into_server_error())
        })
    }
}
