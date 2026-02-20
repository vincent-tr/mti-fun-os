use libruntime::{
    ipc::Handle,
    vfs::{
        iface::VfsServerError,
        types::{HandlePermissions, NodeType},
    },
};

use crate::vnode::VNode;

/// Represents an opened node (file, directory, or symlink) in the VFS server.
#[derive(Debug)]
pub struct OpenedNode {
    vnode: VNode,
    r#type: NodeType,
    handle_permissions: HandlePermissions,
    fs_handle: Option<Handle>,
}

impl OpenedNode {
    /// Creates a new OpenedNode with the given vnode and handle permissions.
    pub async fn new(
        vnode: VNode,
        r#type: NodeType,
        handle_permissions: HandlePermissions,
        fs_handle: Option<Handle>,
    ) -> Self {
        let obj = Self {
            vnode,
            r#type,
            handle_permissions,
            fs_handle,
        };

        obj.vnode.mount_link().await;

        obj
    }

    /// Marks the opened node as closed, unlinking its mount to allow it to be unmounted when no longer in use.
    pub async fn mark_closed(&self) {
        self.vnode.mount_unlink().await;
    }

    /// Returns a reference to the vnode associated with this opened node.
    pub fn vnode(&self) -> VNode {
        self.vnode
    }

    /// Returns the filesystem handle associated with this opened node, if any.
    pub fn fs_handle(&self) -> Option<Handle> {
        self.fs_handle
    }

    /// Checks if the opened node has read permissions.
    pub fn can_read(&self) -> bool {
        self.handle_permissions.contains(HandlePermissions::READ)
    }

    /// Checks if the opened node has write permissions.
    pub fn can_write(&self) -> bool {
        self.handle_permissions.contains(HandlePermissions::WRITE)
    }

    /// Returns the type of the opened node (file, directory, or symlink).
    pub fn r#type(&self) -> NodeType {
        self.r#type
    }

    /// Checks if the opened node has read permissions.
    pub fn check_read(&self) -> Result<(), VfsServerError> {
        if self.can_read() {
            Ok(())
        } else {
            Err(VfsServerError::AccessDenied)
        }
    }

    /// Checks if the opened node has write permissions.
    pub fn check_write(&self) -> Result<(), VfsServerError> {
        if self.can_write() {
            Ok(())
        } else {
            Err(VfsServerError::AccessDenied)
        }
    }

    /// Checks if the opened node is of the expected type.
    pub fn check_type(&self, expected: NodeType) -> Result<(), VfsServerError> {
        if self.r#type == expected {
            Ok(())
        } else {
            Err(VfsServerError::BadType)
        }
    }
}
