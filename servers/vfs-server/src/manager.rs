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

        let builder = builder.with_handler(messages::Type::Open, Self::open_handler);
        let builder = builder.with_handler(messages::Type::Close, Self::close_handler);
        let builder = builder.with_handler(messages::Type::Stat, Self::stat_handler);
        let builder = builder.with_handler(
            messages::Type::SetPermissions,
            Self::set_permissions_handler,
        );
        let builder = builder.with_handler(messages::Type::Read, Self::read_handler);
        let builder = builder.with_handler(messages::Type::Write, Self::write_handler);
        let builder = builder.with_handler(messages::Type::Resize, Self::resize_handler);
        let builder = builder.with_handler(messages::Type::List, Self::list_handler);
        let builder = builder.with_handler(messages::Type::Move, Self::move_handler);
        let builder = builder.with_handler(messages::Type::Remove, Self::remove_handler);
        let builder =
            builder.with_handler(messages::Type::CreateSymlink, Self::create_symlink_handler);
        let builder = builder.with_handler(messages::Type::ReadSymlink, Self::read_symlink_handler);
        let builder = builder.with_handler(messages::Type::Mount, Self::mount_handler);
        let builder = builder.with_handler(messages::Type::Unmount, Self::unmount_handler);
        let builder = builder.with_handler(messages::Type::ListMounts, Self::list_mounts_handler);

        builder.build()
    }

    async fn process_terminated(self: Arc<Self>, pid: u64) {
        panic!("not implemented");
    }

    async fn open_handler(
        self: Arc<Self>,
        query: messages::OpenQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::OpenReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn close_handler(
        self: Arc<Self>,
        query: messages::CloseQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CloseReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn stat_handler(
        self: Arc<Self>,
        query: messages::StatQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::StatReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn set_permissions_handler(
        self: Arc<Self>,
        query: messages::SetPermissionsQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::SetPermissionsReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn read_handler(
        self: Arc<Self>,
        query: messages::ReadQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ReadReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn write_handler(
        self: Arc<Self>,
        query: messages::WriteQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::WriteReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn resize_handler(
        self: Arc<Self>,
        query: messages::ResizeQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ResizeReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn list_handler(
        self: Arc<Self>,
        query: messages::ListQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn move_handler(
        self: Arc<Self>,
        query: messages::MoveQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::MoveReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn remove_handler(
        self: Arc<Self>,
        query: messages::RemoveQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::RemoveReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn create_symlink_handler(
        self: Arc<Self>,
        query: messages::CreateSymlinkQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CreateSymlinkReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn read_symlink_handler(
        self: Arc<Self>,
        query: messages::ReadSymlinkQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ReadSymlinkReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn mount_handler(
        self: Arc<Self>,
        query: messages::MountQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::MountReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn unmount_handler(
        self: Arc<Self>,
        query: messages::UnmountQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::UnmountReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }

    async fn list_mounts_handler(
        self: Arc<Self>,
        query: messages::ListMountsQueryParameters,
        query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListMountsReply, ipc::KHandles), InternalError> {
        panic!("not implemented");
    }
}
