use core::fmt;

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use async_trait::async_trait;
use log::error;

use crate::{
    ipc::{self, Handle},
    kobject,
    vfs::types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions},
};

use super::{
    dentries_block::DentriesBlock, messages, mounts_block::MountsBlock, DirectoryEntry, MountInfo,
    VfsServerError,
};

/// VFS server interface
#[async_trait]
pub trait VfsServer: Send + Sync {
    type Error: Into<VfsServerError>;

    async fn process_terminated(&self, _pid: u64) {}

    async fn open(
        &self,
        sender_id: u64,
        path: &str,
        r#type: Option<NodeType>,
        mode: OpenMode,
        no_follow: bool,
        permissions: Permissions,
        handle_permissions: HandlePermissions,
    ) -> Result<Handle, Self::Error>;

    async fn close(&self, sender_id: u64, handle: Handle) -> Result<(), Self::Error>;

    async fn stat(&self, sender_id: u64, handle: Handle) -> Result<Metadata, Self::Error>;

    async fn set_permissions(
        &self,
        sender_id: u64,
        handle: Handle,
        permissions: Permissions,
    ) -> Result<(), Self::Error>;

    async fn read(
        &self,
        sender_id: u64,
        handle: Handle,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error>;

    async fn write(
        &self,
        sender_id: u64,
        handle: Handle,
        offset: usize,
        buffer: &[u8],
    ) -> Result<usize, Self::Error>;

    async fn resize(
        &self,
        sender_id: u64,
        handle: Handle,
        new_size: usize,
    ) -> Result<(), Self::Error>;

    async fn list(
        &self,
        sender_id: u64,
        handle: Handle,
    ) -> Result<Vec<DirectoryEntry>, Self::Error>;

    async fn r#move(
        &self,
        sender_id: u64,
        old_dir: Handle,
        old_name: &str,
        new_dir: Handle,
        new_name: &str,
    ) -> Result<(), Self::Error>;

    async fn remove(&self, sender_id: u64, dir: Handle, name: &str) -> Result<(), Self::Error>;

    async fn create_symlink(
        &self,
        sender_id: u64,
        path: &str,
        target: &str,
    ) -> Result<Handle, Self::Error>;

    async fn read_symlink(&self, sender_id: u64, handle: Handle) -> Result<String, Self::Error>;

    async fn mount(
        &self,
        sender_id: u64,
        mount_point: &str,
        fs_port_name: &str,
        args: &[u8],
    ) -> Result<(), Self::Error>;

    async fn unmount(&self, sender_id: u64, mount_point: &str) -> Result<(), Self::Error>;

    async fn list_mounts(&self, sender_id: u64) -> Result<Vec<MountInfo>, Self::Error>;
}

/// The main server structure
#[derive(Debug)]
pub struct Server<Impl: VfsServer + 'static> {
    inner: Impl,
}

impl<Impl: VfsServer + 'static> Server<Impl> {
    pub fn new(inner: Impl) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::AsyncServer, kobject::Error> {
        let builder = ipc::ManagedAsyncServerBuilder::<_, VfsServerError, VfsServerError>::new(
            &self,
            messages::PORT_NAME,
            messages::VERSION,
        );
        let builder = builder.with_process_exit_handler(Self::process_terminated_handler);

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

    async fn process_terminated_handler(self: Arc<Self>, pid: u64) {
        self.inner.process_terminated(pid).await;
    }

    async fn open_handler(
        self: Arc<Self>,
        query: messages::OpenQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::OpenReply, ipc::KHandles), VfsServerError> {
        let path_view = {
            let handle = query_handles.take(messages::OpenQueryParameters::HANDLE_PATH_MOBJ);
            ipc::BufferView::new(handle, &query.path, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create path buffer reader")?
        };

        let path = unsafe { path_view.str() };

        let handle = self
            .inner
            .open(
                sender_id,
                path,
                query.r#type,
                query.mode,
                query.no_follow,
                query.permissions,
                query.handle_permissions,
            )
            .await
            .map_err(Into::into)?;

        Ok((messages::OpenReply { handle }, ipc::KHandles::new()))
    }

    async fn close_handler(
        self: Arc<Self>,
        query: messages::CloseQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CloseReply, ipc::KHandles), VfsServerError> {
        self.inner
            .close(sender_id, query.handle)
            .await
            .map_err(Into::into)?;

        Ok((messages::CloseReply {}, ipc::KHandles::new()))
    }

    async fn stat_handler(
        self: Arc<Self>,
        query: messages::StatQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::StatReply, ipc::KHandles), VfsServerError> {
        let metadata = self
            .inner
            .stat(sender_id, query.handle)
            .await
            .map_err(Into::into)?;

        Ok((messages::StatReply { metadata }, ipc::KHandles::new()))
    }

    async fn set_permissions_handler(
        self: Arc<Self>,
        query: messages::SetPermissionsQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::SetPermissionsReply, ipc::KHandles), VfsServerError> {
        self.inner
            .set_permissions(sender_id, query.handle, query.permissions)
            .await
            .map_err(Into::into)?;

        Ok((messages::SetPermissionsReply {}, ipc::KHandles::new()))
    }

    async fn read_handler(
        self: Arc<Self>,
        query: messages::ReadQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ReadReply, ipc::KHandles), VfsServerError> {
        let mut buffer_view = {
            let handle = query_handles.take(messages::ReadQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create read buffer view")?
        };

        let bytes_read = self
            .inner
            .read(
                sender_id,
                query.handle,
                query.offset,
                buffer_view.buffer_mut(),
            )
            .await
            .map_err(Into::into)?;

        Ok((messages::ReadReply { bytes_read }, ipc::KHandles::new()))
    }

    async fn write_handler(
        self: Arc<Self>,
        query: messages::WriteQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::WriteReply, ipc::KHandles), VfsServerError> {
        let buffer_view = {
            let handle = query_handles.take(messages::WriteQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create write buffer view")?
        };

        let bytes_written = self
            .inner
            .write(sender_id, query.handle, query.offset, buffer_view.buffer())
            .await
            .map_err(Into::into)?;

        Ok((messages::WriteReply { bytes_written }, ipc::KHandles::new()))
    }

    async fn resize_handler(
        self: Arc<Self>,
        query: messages::ResizeQueryParameters,
        _query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ResizeReply, ipc::KHandles), VfsServerError> {
        self.inner
            .resize(sender_id, query.handle, query.new_size)
            .await
            .map_err(Into::into)?;

        Ok((messages::ResizeReply {}, ipc::KHandles::new()))
    }

    async fn list_handler(
        self: Arc<Self>,
        query: messages::ListQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListReply, ipc::KHandles), VfsServerError> {
        let mut buffer_view = {
            let handle = query_handles.take(messages::ListQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create directory list buffer view")?
        };

        let entries = self
            .inner
            .list(sender_id, query.handle)
            .await
            .map_err(Into::into)?;

        let buffer_used_len =
            DentriesBlock::build(&entries, buffer_view.buffer_mut()).map_err(
                |required_size| {
                    error!("Provided buffer too small for directory list ({} bytes needed, {} bytes provided)",
                        required_size,
                        buffer_view.buffer().len()
                    );
                    VfsServerError::BufferTooSmall
                },
            )?;

        Ok((
            messages::ListReply { buffer_used_len },
            ipc::KHandles::new(),
        ))
    }

    async fn move_handler(
        self: Arc<Self>,
        query: messages::MoveQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::MoveReply, ipc::KHandles), VfsServerError> {
        let old_name_view = {
            let handle = query_handles.take(messages::MoveQueryParameters::HANDLE_OLD_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.old_name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create old name buffer view")?
        };

        let new_name_view = {
            let handle = query_handles.take(messages::MoveQueryParameters::HANDLE_NEW_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.new_name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create new name buffer view")?
        };

        let old_name = unsafe { old_name_view.str() };
        let new_name = unsafe { new_name_view.str() };

        self.inner
            .r#move(sender_id, query.old_dir, old_name, query.new_dir, new_name)
            .await
            .map_err(Into::into)?;

        Ok((messages::MoveReply {}, ipc::KHandles::new()))
    }

    async fn remove_handler(
        self: Arc<Self>,
        query: messages::RemoveQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::RemoveReply, ipc::KHandles), VfsServerError> {
        let name_view = {
            let handle = query_handles.take(messages::RemoveQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer view")?
        };

        let name = unsafe { name_view.str() };

        self.inner
            .remove(sender_id, query.dir, name)
            .await
            .map_err(Into::into)?;

        Ok((messages::RemoveReply {}, ipc::KHandles::new()))
    }

    async fn create_symlink_handler(
        self: Arc<Self>,
        query: messages::CreateSymlinkQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::CreateSymlinkReply, ipc::KHandles), VfsServerError> {
        let path_view = {
            let handle =
                query_handles.take(messages::CreateSymlinkQueryParameters::HANDLE_PATH_MOBJ);
            ipc::BufferView::new(handle, &query.path, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create path buffer view")?
        };

        let target_view = {
            let handle =
                query_handles.take(messages::CreateSymlinkQueryParameters::HANDLE_TARGET_MOBJ);
            ipc::BufferView::new(handle, &query.target, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create target buffer view")?
        };

        let path = unsafe { path_view.str() };
        let target = unsafe { target_view.str() };

        let handle = self
            .inner
            .create_symlink(sender_id, path, target)
            .await
            .map_err(Into::into)?;

        Ok((
            messages::CreateSymlinkReply { handle },
            ipc::KHandles::new(),
        ))
    }

    async fn read_symlink_handler(
        self: Arc<Self>,
        query: messages::ReadSymlinkQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ReadSymlinkReply, ipc::KHandles), VfsServerError> {
        let mut buffer_view = {
            let handle =
                query_handles.take(messages::ReadSymlinkQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create read buffer view")?
        };

        let target_path = self
            .inner
            .read_symlink(sender_id, query.handle)
            .await
            .map_err(Into::into)?;

        let target_path_bytes = target_path.as_bytes();
        if buffer_view.buffer().len() < target_path_bytes.len() {
            error!(
                "Provided buffer too small for symlink target path ({} bytes needed, {} bytes provided)",
                target_path_bytes.len(),
                buffer_view.buffer().len()
            );
            return Err(VfsServerError::BufferTooSmall);
        }

        buffer_view.buffer_mut()[..target_path_bytes.len()].copy_from_slice(target_path_bytes);

        Ok((
            messages::ReadSymlinkReply {
                target_length: target_path.len(),
            },
            ipc::KHandles::new(),
        ))
    }

    async fn mount_handler(
        self: Arc<Self>,
        query: messages::MountQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::MountReply, ipc::KHandles), VfsServerError> {
        let mount_point_view = {
            let handle =
                query_handles.take(messages::MountQueryParameters::HANDLE_MOUNT_POINT_MOBJ);
            ipc::BufferView::new(handle, &query.mount_point, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create mount point buffer view")?
        };

        let fs_port_name_view = {
            let handle =
                query_handles.take(messages::MountQueryParameters::HANDLE_FS_PORT_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.fs_port_name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create fs port name buffer view")?
        };

        let args_view = {
            let handle = query_handles.take(messages::MountQueryParameters::HANDLE_ARGS_MOBJ);
            ipc::BufferView::new(handle, &query.args, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create mount args buffer view")?
        };

        let mount_point = unsafe { mount_point_view.str() };
        let fs_port_name = unsafe { fs_port_name_view.str() };

        self.inner
            .mount(sender_id, mount_point, fs_port_name, args_view.buffer())
            .await
            .map_err(Into::into)?;

        Ok((messages::MountReply {}, ipc::KHandles::new()))
    }

    async fn unmount_handler(
        self: Arc<Self>,
        query: messages::UnmountQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::UnmountReply, ipc::KHandles), VfsServerError> {
        let mount_point_view = {
            let handle =
                query_handles.take(messages::UnmountQueryParameters::HANDLE_MOUNT_POINT_MOBJ);
            ipc::BufferView::new(handle, &query.mount_point, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create mount point buffer view")?
        };

        let mount_point = unsafe { mount_point_view.str() };

        self.inner
            .unmount(sender_id, mount_point)
            .await
            .map_err(Into::into)?;

        Ok((messages::UnmountReply {}, ipc::KHandles::new()))
    }

    async fn list_mounts_handler(
        self: Arc<Self>,
        query: messages::ListMountsQueryParameters,
        mut query_handles: ipc::KHandles,
        sender_id: u64,
    ) -> Result<(messages::ListMountsReply, ipc::KHandles), VfsServerError> {
        let mut buffer_view = {
            let handle =
                query_handles.take(messages::ListMountsQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create mounts list buffer view")?
        };

        let mounts = self
            .inner
            .list_mounts(sender_id)
            .await
            .map_err(Into::into)?;

        let buffer_used_len =
            MountsBlock::build(&mounts, buffer_view.buffer_mut()).map_err(|required_size| {
                error!("Provided buffer too small for mounts list ({} bytes needed, {} bytes provided)",
                    required_size,
                    buffer_view.buffer().len()
                );
                VfsServerError::BufferTooSmall
            })?;

        Ok((
            messages::ListMountsReply { buffer_used_len },
            ipc::KHandles::new(),
        ))
    }
}

/// Extension trait for Result to add context
trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, VfsServerError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, VfsServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            VfsServerError::InvalidArgument
        })
    }
}
