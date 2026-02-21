use alloc::{string::String, sync::Arc, vec::Vec};
use libruntime::{
    ipc::Handle,
    vfs::{
        DirectoryEntry, HandlePermissions, NodeType, Permissions,
        iface::VfsServerError,
        types::{Metadata, NodeId},
    },
};

use crate::{
    cache::{LookupCache, NodeAttributesCache},
    mounts::{Mount, MountId, MountTable},
};

/// A vnode represents a node in the virtual file system, which is identified by its mount point and node ID.
///
/// Note: The VNode API takes care of caching metadata and lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VNode {
    /// The mount point of the vnode, which identifies the file system it belongs to.
    mount: MountId,

    /// The node ID of the vnode, which identifies the node within the file system.
    node: NodeId,
}

impl VNode {
    /// The root vnode, representing the root of the entire virtual file system.
    pub const ROOT: VNode = VNode {
        mount: MountId::ROOT,
        node: NodeId::ROOT,
    };

    /// Create a new vnode with the given mount point and node ID.
    pub fn new(mount: MountId, node: NodeId) -> Self {
        Self { mount, node }
    }

    /// Get the mount ID of the vnode.
    pub fn mount_id(&self) -> MountId {
        self.mount
    }

    /// Get the node ID of the vnode.
    pub fn node_id(&self) -> NodeId {
        self.node
    }

    /// Get the mount point of the vnode.
    async fn mount(&self) -> Arc<Mount> {
        MountTable::get()
            .get_mount(self.mount)
            .await
            .expect("Mount not found")
    }

    /// Link the mount point of the vnode, increasing its reference count to prevent it from being unmounted while in use.
    pub async fn mount_link(&self) {
        self.mount().await.link();
    }

    /// Unlink the mount point of the vnode, decreasing its reference count to allow it to be unmounted when no longer in use.
    pub async fn mount_unlink(&self) {
        self.mount().await.unlink();
    }

    /// Get the type of the vnode.
    pub async fn r#type(&self) -> Result<NodeType, VfsServerError> {
        if let Some(attributes) = NodeAttributesCache::get().fetch(self) {
            return Ok(attributes.r#type());
        }

        let metadata = self.get_metadata().await?;
        NodeAttributesCache::get().update(*self, metadata.r#type, metadata.permissions);
        Ok(metadata.r#type)
    }

    /// Get the permissions of the vnode.
    pub async fn permissions(&self) -> Result<Permissions, VfsServerError> {
        if let Some(attributes) = NodeAttributesCache::get().fetch(self) {
            return Ok(attributes.permissions());
        }

        let metadata = self.get_metadata().await?;
        NodeAttributesCache::get().update(*self, metadata.r#type, metadata.permissions);
        Ok(metadata.permissions)
    }

    /// Lookup a node in the mounted file system by its node ID, returning a vnode that can be used to access the node.
    pub async fn lookup(&self, name: &str) -> Result<VNode, VfsServerError> {
        if let Some(vnode) = LookupCache::get().fetch(*self, name) {
            return Ok(vnode);
        }

        let node_id = self.mount().await.lookup(self.node_id(), name).await?;

        let vnode = VNode::new(self.mount_id(), node_id);
        LookupCache::get().update(*self, name, vnode);
        Ok(vnode)
    }

    /// Create a new node in the mounted file system, returning a vnode that can be used to access the node.
    pub async fn create(
        &self,
        name: &str,
        r#type: NodeType,
        permissions: Permissions,
    ) -> Result<VNode, VfsServerError> {
        let node_id = self
            .mount()
            .await
            .create(self.node_id(), name, r#type, permissions)
            .await?;

        let vnode = VNode::new(self.mount_id(), node_id);
        LookupCache::get().update(*self, name, vnode);
        NodeAttributesCache::get().update(vnode, r#type, permissions);
        Ok(vnode)
    }

    /// Remove a node from the mounted file system by its node ID and name.
    pub async fn remove(&self, name: &str) -> Result<(), VfsServerError> {
        let node_id = self.mount().await.remove(self.node_id(), name).await?;

        LookupCache::get().remove(*self, name);
        NodeAttributesCache::get().remove(VNode::new(self.mount_id(), node_id));
        Ok(())
    }

    /// Move a node from one location to another in the mounted file system by their parent node IDs and names.
    pub async fn r#move(
        &self,
        src_name: &str,
        dst_parent: NodeId,
        dst_name: &str,
    ) -> Result<(), VfsServerError> {
        let node_id = self
            .mount()
            .await
            .r#move(self.node_id(), src_name, dst_parent, dst_name)
            .await?;

        let dst_parent = VNode::new(self.mount_id(), dst_parent);
        let cache = LookupCache::get();
        cache.remove(*self, src_name);
        cache.update(dst_parent, dst_name, VNode::new(self.mount_id(), node_id));

        Ok(())
    }

    /// Get the metadata of a node in the mounted file system.
    pub async fn get_metadata(&self) -> Result<Metadata, VfsServerError> {
        self.mount().await.get_metadata(self.node_id()).await
    }

    /// Set the metadata of a node in the mounted file system.
    pub async fn set_metadata(
        &self,
        permissions: Option<Permissions>,
        size: Option<usize>,
        created: Option<u64>,
        modified: Option<u64>,
    ) -> Result<(), VfsServerError> {
        self.mount()
            .await
            .set_metadata(self.node_id(), permissions, size, created, modified)
            .await?;

        // Update the cache if permissions or type is changed, as they are cached in NodeAttributesCache.
        if permissions.is_some() {
            let r#type = self.r#type().await?;
            NodeAttributesCache::get().update(*self, r#type, permissions.unwrap());
        }

        Ok(())
    }

    /// Open a file in the mounted file system by its node ID and return a handle that can be used to access the file.
    pub async fn open_file(
        &self,
        open_permissions: HandlePermissions,
    ) -> Result<Handle, VfsServerError> {
        self.mount()
            .await
            .open_file(self.node_id(), open_permissions)
            .await
    }

    /// Close a file in the mounted file system by its handle.
    pub async fn close_file(&self, handle: Handle) -> Result<(), VfsServerError> {
        self.mount().await.close_file(handle).await
    }

    /// Read data from a file in the mounted file system by its handle.
    pub async fn read_file(
        &self,
        handle: Handle,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, VfsServerError> {
        self.mount().await.read_file(handle, offset, buffer).await
    }

    /// Write data to a file in the mounted file system by its handle.
    pub async fn write_file(
        &self,
        handle: Handle,
        offset: usize,
        buffer: &[u8],
    ) -> Result<usize, VfsServerError> {
        self.mount().await.write_file(handle, offset, buffer).await
    }

    /// Open a directory in the mounted file system by its node ID and return a handle that can be used to access the directory.
    pub async fn open_dir(&self) -> Result<Handle, VfsServerError> {
        self.mount().await.open_dir(self.node_id()).await
    }

    /// Close a directory in the mounted file system by its handle.
    pub async fn close_dir(&self, handle: Handle) -> Result<(), VfsServerError> {
        self.mount().await.close_dir(handle).await
    }

    /// List the entries in a directory in the mounted file system by its handle.
    pub async fn list_dir(&self, handle: Handle) -> Result<Vec<DirectoryEntry>, VfsServerError> {
        self.mount().await.list_dir(handle).await
    }

    /// Create a symbolic link in the mounted file system by its parent node ID, name, and target path.
    pub async fn create_symlink(&self, name: &str, target: &str) -> Result<VNode, VfsServerError> {
        let node_id = self
            .mount()
            .await
            .create_symlink(self.node_id(), name, target)
            .await?;

        let vnode = VNode::new(self.mount_id(), node_id);
        LookupCache::get().update(*self, name, vnode);
        NodeAttributesCache::get().update(
            vnode,
            NodeType::Symlink,
            Permissions::READ | Permissions::WRITE | Permissions::EXECUTE,
        );
        Ok(vnode)
    }

    /// Read the target path of a symbolic link in the mounted file system by its node ID.
    pub async fn read_symlink(&self) -> Result<String, VfsServerError> {
        self.mount().await.read_symlink(self.node_id()).await
    }
}
