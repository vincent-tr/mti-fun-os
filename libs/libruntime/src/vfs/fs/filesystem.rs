use async_trait::async_trait;

use super::messages::{FsServerError, NodeId};
use crate::{
    ipc::Handle,
    vfs::types::{Metadata, NodeType, Permissions},
};
use alloc::boxed::Box;

// TODO: string writer, buffer writer/reader

#[async_trait]
pub trait FileSystem: Send + Sync {
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
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error>;

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
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Mount a filesystem.
    async fn mount(&self, args: &[u8]) -> Result<Handle, Self::Error>;

    /// Unmount a filesystem.
    async fn unmount(&self, mount_handle: Handle) -> Result<(), Self::Error>;
}
