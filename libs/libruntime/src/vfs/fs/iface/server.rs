use core::fmt;

use async_trait::async_trait;
use log::error;

use super::{messages, DentriesBlock, DirectoryEntry, FsServerError};
use crate::{
    ipc::{self, Handle},
    kobject,
    vfs::types::{Metadata, NodeId, NodeType, Permissions},
};
use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};

/// Filesystem server trait that must be implemented by any filesystem server.
#[async_trait]
pub trait FileSystem: Send + Sync + fmt::Debug {
    type Error: Into<FsServerError>;

    /// Look up a child node by name.
    async fn lookup(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
    ) -> Result<NodeId, Self::Error>;

    /// Create a new file or directory.
    async fn create(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
        node_type: NodeType,
        permissions: Permissions,
    ) -> Result<NodeId, Self::Error>;

    /// Remove a file or directory.
    async fn remove(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
    ) -> Result<(), Self::Error>;

    /// Move a file or directory.
    async fn r#move(
        &self,
        mount_handle: Handle,
        src_parent: NodeId,
        src_name: &str,
        dst_parent: NodeId,
        dst_name: &str,
    ) -> Result<(), Self::Error>;

    /// Get metadata of a node.
    async fn get_metadata(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<Metadata, Self::Error>;

    /// Set metadata of a node.
    async fn set_metadata(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
        permissions: Option<Permissions>,
        size: Option<usize>,
        created: Option<u64>,
        modified: Option<u64>,
    ) -> Result<(), Self::Error>;

    /// Open a file.
    async fn open_file(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
        open_permissions: Permissions,
    ) -> Result<Handle, Self::Error>;

    /// Close a file.
    async fn close_file(&self, mount_handle: Handle, handle: Handle) -> Result<(), Self::Error>;

    /// Read from a file.
    async fn read_file(
        &self,
        mount_handle: Handle,
        handle: Handle,
        buffer: &mut [u8],
        offset: usize,
    ) -> Result<usize, Self::Error>;

    /// Write to a file.
    async fn write_file(
        &self,
        mount_handle: Handle,
        handle: Handle,
        buffer: &[u8],
        offset: usize,
    ) -> Result<usize, Self::Error>;

    /// Open a directory.    
    async fn open_dir(&self, mount_handle: Handle, node_id: NodeId) -> Result<Handle, Self::Error>;

    /// Close a directory.
    async fn close_dir(&self, mount_handle: Handle, handle: Handle) -> Result<(), Self::Error>;

    /// Read entries from a directory.
    async fn list_dir(
        &self,
        mount_handle: Handle,
        handle: Handle,
    ) -> Result<Vec<DirectoryEntry>, Self::Error>;

    /// Create a symbolic link.
    async fn create_symlink(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
        target: &str,
    ) -> Result<NodeId, Self::Error>;

    /// Read the target of a symbolic link.
    async fn read_symlink(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<String, Self::Error>;

    /// Mount a filesystem.
    async fn mount(&self, args: &[u8]) -> Result<(Handle, NodeId), Self::Error>;

    /// Unmount a filesystem.
    async fn unmount(&self, mount_handle: Handle) -> Result<(), Self::Error>;
}

/// The main server structure
#[derive(Debug)]
pub struct Server<Impl: FileSystem + 'static> {
    inner: Impl,
}

impl<Impl: FileSystem + 'static> Server<Impl> {
    pub fn new(inner: Impl) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub fn build_ipc_server(
        self: &Arc<Self>,
        port_name: &'static str,
    ) -> Result<ipc::AsyncServer, kobject::Error> {
        let builder = ipc::ManagedAsyncServerBuilder::<_, FsServerError, FsServerError>::new(
            &self,
            port_name,
            messages::VERSION,
        );

        let builder = builder.with_handler(messages::Type::Lookup, Self::lookup_handler);
        let builder = builder.with_handler(messages::Type::Create, Self::create_handler);
        let builder = builder.with_handler(messages::Type::Remove, Self::remove_handler);
        let builder = builder.with_handler(messages::Type::Move, Self::move_handler);
        let builder = builder.with_handler(messages::Type::GetMetadata, Self::get_metadata_handler);
        let builder = builder.with_handler(messages::Type::SetMetadata, Self::set_metadata_handler);
        let builder = builder.with_handler(messages::Type::OpenFile, Self::open_file_handler);
        let builder = builder.with_handler(messages::Type::CloseFile, Self::close_file_handler);
        let builder = builder.with_handler(messages::Type::ReadFile, Self::read_file_handler);
        let builder = builder.with_handler(messages::Type::WriteFile, Self::write_file_handler);
        let builder = builder.with_handler(messages::Type::OpenDir, Self::open_dir_handler);
        let builder = builder.with_handler(messages::Type::CloseDir, Self::close_dir_handler);
        let builder = builder.with_handler(messages::Type::ListDir, Self::list_dir_handler);
        let builder =
            builder.with_handler(messages::Type::CreateSymlink, Self::create_symlink_handler);
        let builder = builder.with_handler(messages::Type::ReadSymlink, Self::read_symlink_handler);
        let builder = builder.with_handler(messages::Type::Mount, Self::mount_handler);
        let builder = builder.with_handler(messages::Type::Unmount, Self::unmount_handler);

        builder.build()
    }

    async fn lookup_handler(
        self: Arc<Self>,
        query: messages::LookupQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::LookupReply, ipc::KHandles), FsServerError> {
        let name_view = {
            let handle = query_handles.take(messages::LookupQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let name = unsafe { name_view.str() };

        let node_id = self
            .inner
            .lookup(query.mount_handle, query.parent, name)
            .await
            .map_err(Into::into)?;

        Ok((messages::LookupReply { node_id }, ipc::KHandles::new()))
    }

    async fn create_handler(
        self: Arc<Self>,
        query: messages::CreateQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::CreateReply, ipc::KHandles), FsServerError> {
        let name_view = {
            let handle = query_handles.take(messages::CreateQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let name = unsafe { name_view.str() };

        let node_id = self
            .inner
            .create(
                query.mount_handle,
                query.parent,
                name,
                query.r#type,
                query.permissions,
            )
            .await
            .map_err(Into::into)?;

        Ok((messages::CreateReply { node_id }, ipc::KHandles::new()))
    }

    async fn remove_handler(
        self: Arc<Self>,
        query: messages::RemoveQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::RemoveReply, ipc::KHandles), FsServerError> {
        let name_view = {
            let handle = query_handles.take(messages::RemoveQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let name = unsafe { name_view.str() };

        self.inner
            .remove(query.mount_handle, query.parent, name)
            .await
            .map_err(Into::into)?;

        Ok((messages::RemoveReply {}, ipc::KHandles::new()))
    }

    async fn move_handler(
        self: Arc<Self>,
        query: messages::MoveQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::MoveReply, ipc::KHandles), FsServerError> {
        let src_name_view = {
            let handle = query_handles.take(messages::MoveQueryParameters::HANDLE_SRC_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.src_name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create source name buffer reader")?
        };

        let dst_name_view = {
            let handle = query_handles.take(messages::MoveQueryParameters::HANDLE_DST_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.dst_name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create destination name buffer reader")?
        };

        let src_name = unsafe { src_name_view.str() };
        let dst_name = unsafe { dst_name_view.str() };

        self.inner
            .r#move(
                query.mount_handle,
                query.src_parent,
                src_name,
                query.dst_parent,
                dst_name,
            )
            .await
            .map_err(Into::into)?;

        Ok((messages::MoveReply {}, ipc::KHandles::new()))
    }

    async fn get_metadata_handler(
        self: Arc<Self>,
        query: messages::GetMetadataQueryParameters,
        _query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::GetMetadataReply, ipc::KHandles), FsServerError> {
        let metadata = self
            .inner
            .get_metadata(query.mount_handle, query.node_id)
            .await
            .map_err(Into::into)?;

        Ok((
            messages::GetMetadataReply { metadata },
            ipc::KHandles::new(),
        ))
    }

    async fn set_metadata_handler(
        self: Arc<Self>,
        query: messages::SetMetadataQueryParameters,
        _query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::SetMetadataReply, ipc::KHandles), FsServerError> {
        self.inner
            .set_metadata(
                query.mount_handle,
                query.node_id,
                query.permissions,
                query.size,
                query.created,
                query.modified,
            )
            .await
            .map_err(Into::into)?;

        Ok((messages::SetMetadataReply {}, ipc::KHandles::new()))
    }

    async fn open_file_handler(
        self: Arc<Self>,
        query: messages::OpenFileQueryParameters,
        _query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::OpenFileReply, ipc::KHandles), FsServerError> {
        let handle = self
            .inner
            .open_file(query.mount_handle, query.node_id, query.open_permissions)
            .await
            .map_err(Into::into)?;

        Ok((messages::OpenFileReply { handle }, ipc::KHandles::new()))
    }

    async fn close_file_handler(
        self: Arc<Self>,
        query: messages::CloseFileQueryParameters,
        _query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::CloseFileReply, ipc::KHandles), FsServerError> {
        self.inner
            .close_file(query.mount_handle, query.handle)
            .await
            .map_err(Into::into)?;

        Ok((messages::CloseFileReply {}, ipc::KHandles::new()))
    }

    async fn read_file_handler(
        self: Arc<Self>,
        query: messages::ReadFileQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::ReadFileReply, ipc::KHandles), FsServerError> {
        let mut buffer_view = {
            let handle = query_handles.take(messages::ReadFileQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let bytes_read = self
            .inner
            .read_file(
                query.mount_handle,
                query.handle,
                buffer_view.buffer_mut(),
                query.offset,
            )
            .await
            .map_err(Into::into)?;

        Ok((messages::ReadFileReply { bytes_read }, ipc::KHandles::new()))
    }

    async fn write_file_handler(
        self: Arc<Self>,
        query: messages::WriteFileQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::WriteFileReply, ipc::KHandles), FsServerError> {
        let buffer_view = {
            let handle = query_handles.take(messages::WriteFileQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let bytes_written = self
            .inner
            .write_file(
                query.mount_handle,
                query.handle,
                buffer_view.buffer(),
                query.offset,
            )
            .await
            .map_err(Into::into)?;

        Ok((
            messages::WriteFileReply { bytes_written },
            ipc::KHandles::new(),
        ))
    }

    async fn open_dir_handler(
        self: Arc<Self>,
        query: messages::OpenDirQueryParameters,
        _query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::OpenDirReply, ipc::KHandles), FsServerError> {
        let handle = self
            .inner
            .open_dir(query.mount_handle, query.node_id)
            .await
            .map_err(Into::into)?;

        Ok((messages::OpenDirReply { handle }, ipc::KHandles::new()))
    }

    async fn close_dir_handler(
        self: Arc<Self>,
        query: messages::CloseDirQueryParameters,
        _query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::CloseDirReply, ipc::KHandles), FsServerError> {
        self.inner
            .close_dir(query.mount_handle, query.handle)
            .await
            .map_err(Into::into)?;

        Ok((messages::CloseDirReply {}, ipc::KHandles::new()))
    }

    async fn list_dir_handler(
        self: Arc<Self>,
        query: messages::ListDirQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::ListDirReply, ipc::KHandles), FsServerError> {
        let mut buffer_view = {
            let handle = query_handles.take(messages::ListDirQueryParameters::HANDLE_BUFFER_MOBJ);
            ipc::BufferView::new(handle, &query.buffer, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create buffer view")?
        };

        let entries = self
            .inner
            .list_dir(query.mount_handle, query.handle)
            .await
            .map_err(Into::into)?;

        let buffer_used_len =
            DentriesBlock::build(&entries, buffer_view.buffer_mut()).map_err(
                |required_size| {
                    error!("Provided buffer too small for directory entries ({} bytes needed, {} bytes provided)",
                        required_size,
                        buffer_view.buffer().len()
                    );
                    FsServerError::BufferTooSmall
                },
            )?;

        Ok((
            messages::ListDirReply { buffer_used_len },
            ipc::KHandles::new(),
        ))
    }

    async fn create_symlink_handler(
        self: Arc<Self>,
        query: messages::CreateSymlinkQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::CreateSymlinkReply, ipc::KHandles), FsServerError> {
        let name_view = {
            let handle =
                query_handles.take(messages::CreateSymlinkQueryParameters::HANDLE_NAME_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create name buffer reader")?
        };

        let target_view = {
            let handle =
                query_handles.take(messages::CreateSymlinkQueryParameters::HANDLE_TARGET_MOBJ);
            ipc::BufferView::new(handle, &query.name, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create target buffer reader")?
        };

        let name = unsafe { name_view.str() };
        let target = unsafe { target_view.str() };

        let node_id = self
            .inner
            .create_symlink(query.mount_handle, query.parent, name, target)
            .await
            .map_err(Into::into)?;

        Ok((
            messages::CreateSymlinkReply { node_id },
            ipc::KHandles::new(),
        ))
    }

    async fn read_symlink_handler(
        self: Arc<Self>,
        query: messages::ReadSymlinkQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::ReadSymlinkReply, ipc::KHandles), FsServerError> {
        let mut target_view = {
            let handle =
                query_handles.take(messages::ReadSymlinkQueryParameters::HANDLE_TARGET_MOBJ);
            ipc::BufferView::new(handle, &query.target, ipc::BufferViewAccess::ReadWrite)
                .invalid_arg("Failed to create target view")?
        };

        let target = self
            .inner
            .read_symlink(query.mount_handle, query.node_id)
            .await
            .map_err(Into::into)?;

        if target.len() > target_view.buffer().len() {
            log::error!(
                "Provided buffer too small for process target ({} bytes needed, {} bytes provided)",
                target.len(),
                target_view.buffer().len()
            );
            return Err(FsServerError::BufferTooSmall);
        }

        target_view.buffer_mut()[..target.len()].copy_from_slice(target.as_bytes());

        Ok((
            messages::ReadSymlinkReply {
                target_len: target.len(),
            },
            ipc::KHandles::new(),
        ))
    }

    async fn mount_handler(
        self: Arc<Self>,
        query: messages::MountQueryParameters,
        mut query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::MountReply, ipc::KHandles), FsServerError> {
        let args_view = {
            let handle = query_handles.take(messages::MountQueryParameters::HANDLE_ARGS_MOBJ);
            ipc::BufferView::new(handle, &query.args, ipc::BufferViewAccess::ReadOnly)
                .invalid_arg("Failed to create args buffer reader")?
        };

        let args = args_view.buffer();

        let (mount_handle, root_node_id) = self.inner.mount(args).await.map_err(Into::into)?;

        Ok((
            messages::MountReply {
                mount_handle,
                root_node_id,
            },
            ipc::KHandles::new(),
        ))
    }

    async fn unmount_handler(
        self: Arc<Self>,
        query: messages::UnmountQueryParameters,
        _query_handles: ipc::KHandles,
        _sender_id: u64,
    ) -> Result<(messages::UnmountReply, ipc::KHandles), FsServerError> {
        self.inner
            .unmount(query.mount_handle)
            .await
            .map_err(Into::into)?;

        Ok((messages::UnmountReply {}, ipc::KHandles::new()))
    }
}

/// Extension trait for Result to add context
trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, FsServerError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, FsServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            FsServerError::InvalidArgument
        })
    }
}
