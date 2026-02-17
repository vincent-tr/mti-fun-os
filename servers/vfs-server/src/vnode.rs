use alloc::sync::Arc;
use libruntime::vfs::{
    iface::VfsServerError,
    types::{Metadata, NodeId},
};

use crate::mounts::{Mount, MountId, MountTable};

/// A vnode represents a node in the virtual file system, which is identified by its mount point and node ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VNode {
    /// The mount point of the vnode, which identifies the file system it belongs to.
    mount: MountId,

    /// The node ID of the vnode, which identifies the node within the file system.
    node: NodeId,
}

impl VNode {
    /// Create a new vnode with the given mount point and node ID.
    pub fn new(mount: MountId, node: NodeId) -> Self {
        Self { mount, node }
    }

    /// Get the mount point of the vnode.
    pub fn mount(&self) -> Arc<Mount> {
        MountTable::get()
            .get_mount(self.mount)
            .expect("Mount not found")
    }

    /// Get the metadata of the vnode.
    pub async fn metadata(&self) -> Result<Metadata, VfsServerError> {
        let mount = self.mount();
        mount.get_metadata(self.node).await
    }

    /// Check if the vnode is a mount point.
    pub fn is_mountpoint(&self) -> bool {
        let mount = self.mount();
        *self == mount.root()
    }
}
