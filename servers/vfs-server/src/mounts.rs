use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use alloc::{string::String, sync::Arc, vec::Vec};
use hashbrown::HashMap;
use libruntime::{
    ipc::{CallError, Handle},
    sync::RwLock,
    vfs::{
        fs::iface::{Client, FsServerCallError, FsServerError},
        iface::{DirectoryEntry, MountInfo, VfsServerError},
        types::{Metadata, NodeId, NodeType, Permissions},
    },
};
use log::{error, info};

use crate::vnode::VNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MountId(u64);

/// Mount table, which contains all the mounted file systems and their mount points.
#[derive(Debug)]
pub struct MountTable {
    /// The actual data of the mount table, protected by a read-write lock for concurrent access.
    data: RwLock<MountTableData>,

    /// Generator for unique mount IDs.
    mount_id_generator: AtomicU64,
}

#[derive(Debug)]
struct MountTableData {
    /// List of mounted file systems, indexed by their mount IDs.
    mounts: HashMap<MountId, Arc<Mount>>,

    /// The mount ID of the root file system, which is used to access the root directory.
    root_mount: Option<MountId>,

    /// Mapping from VNode to MountId, which is used to find the mount point of a vnode.
    mountpoints: HashMap<VNode, MountId>,
}

impl MountTable {
    /// Get the global mount table.
    pub fn get() -> &'static MountTable {
        lazy_static::lazy_static! {
            static ref MOUNT_TABLE: MountTable = MountTable::new();
        }

        &MOUNT_TABLE
    }

    /// Create a new mount table.
    fn new() -> Self {
        Self {
            data: RwLock::new(MountTableData {
                mounts: HashMap::new(),
                root_mount: None,
                mountpoints: HashMap::new(),
            }),
            mount_id_generator: AtomicU64::new(0),
        }
    }

    /// Get the root vnode of the file system, which is used to access the root directory.
    pub fn root(&self) -> Option<VNode> {
        let data = self.data.read();
        let root_mount = data.root_mount?;
        let mount = data.mounts.get(&root_mount)?;

        Some(mount.root())
    }

    /// Get a mount point by its ID.
    pub fn get_mount(&self, id: MountId) -> Option<Arc<Mount>> {
        let data = self.data.read();
        Some(data.mounts.get(&id)?.clone())
    }

    /// Lookup if this vnode is a mount point, and if so, return the corresponding mount.
    pub fn get_mountpoint(&self, vnode: &VNode) -> Option<Arc<Mount>> {
        let data = self.data.read();
        let mount_id = data.mountpoints.get(vnode)?;
        Some(data.mounts.get(mount_id).expect("Mount not found").clone())
    }

    /// Mount a file system at the given vnode.
    pub async fn mount(
        &self,
        vnode: &VNode,
        path: String,
        fs_port_name: &str,
        args: &[u8],
    ) -> Result<(), VfsServerError> {
        // Keep the write lock for the entire duration of mounting to prevent any new operations on the mount point while it's being mounted.
        let mut data = self.data.write();

        // Cannot mount on a already mountpoint vnode
        if data.mountpoints.contains_key(vnode) {
            return Err(VfsServerError::InvalidArgument);
        }

        // Cannot mount on the root vnode of a filesystem
        if vnode.mount().root() == *vnode {
            return Err(VfsServerError::InvalidArgument);
        }

        // Can only mount on a directory
        let metadata = vnode.metadata().await?;
        if metadata.r#type != NodeType::Directory {
            return Err(VfsServerError::InvalidArgument);
        }

        let mount_id = self.new_mount_id();
        let mount = Mount::mount(mount_id, path, fs_port_name, args).await?;

        data.mounts.insert(mount_id, mount.clone());
        data.mountpoints.insert(vnode.clone(), mount_id);
        vnode.mount().link();

        info!(
            "Mounted file system {} at vnode {:?} ({}) with mount ID {:?}",
            mount.client.port_name(),
            vnode,
            mount.path(),
            mount.id()
        );

        Ok(())
    }

    /// Unmount a file system at the given vnode.
    pub async fn unmount(&self, vnode: &VNode) -> Result<(), VfsServerError> {
        // Keep the write lock for the entire duration of unmounting to prevent any new operations on the mount point while it's being unmounted.
        let mut data = self.data.write();

        let mount_id = *data
            .mountpoints
            .get(vnode)
            .ok_or(VfsServerError::NotFound)?;

        let mount = data
            .mounts
            .get(&mount_id)
            .clone()
            .expect("Mount not found")
            .clone();

        if mount.link_count.load(Ordering::SeqCst) > 0 {
            return Err(VfsServerError::Busy);
        }

        data.mountpoints.remove(vnode);
        data.mounts.remove(&mount_id);
        vnode.mount().unlink();

        mount.unmount().await?;

        info!(
            "Unmounted file system {} at vnode {:?} ({}) with mount ID {:?}",
            mount.client.port_name(),
            vnode,
            mount.path(),
            mount.id()
        );

        Ok(())
    }

    /// List all the mounted file systems and their mount points.
    pub fn info(&self) -> Vec<MountInfo> {
        let data = self.data.read();
        data.mounts
            .values()
            .map(|mount| MountInfo {
                mount_point: mount.path.clone(),
                fs_port_name: String::from(mount.client.port_name()),
            })
            .collect()
    }

    fn new_mount_id(&self) -> MountId {
        let id = self.mount_id_generator.fetch_add(1, Ordering::SeqCst);
        MountId(id)
    }
}

/// A mount point represents a mounted file system, which contains the file system implementation and its root node.
#[derive(Debug)]
pub struct Mount {
    /// The ID of the mount point, which is used to identify the file system in the VNode.
    id: MountId,

    /// The canonical path of the mount point, which is used for debugging and informational purposes.
    path: String,

    /// The file system implementation of the mount point.
    client: Client<'static>,

    /// Mount handle provided by the client FS at mounting time
    handle: Handle,

    /// The root node of the file system, which is used to access the file system.
    root: NodeId,

    /// The number of links to this mount point, which is used to determine when to unmount the file system.
    link_count: AtomicUsize,
}

impl Mount {
    /// Mount a file system by connecting to the client FS and getting its root node.
    ///
    /// Reserved for MountTable::mount
    async fn mount(
        id: MountId,
        path: String,
        fs_port_name: &str,
        args: &[u8],
    ) -> Result<Arc<Self>, VfsServerError> {
        let client = Client::new(fs_port_name);

        let (handle, root) = client.mount(args).await.map_call_err("Failed to mount")?;

        Ok(Arc::new(Self {
            id,
            path,
            client,
            handle,
            root,
            link_count: AtomicUsize::new(0),
        }))
    }

    /// Unmount the file system, which involves closing the connection to the client FS and cleaning up any resources associated with the mount point.
    ///
    /// Reserved for MountTable::unmount
    async fn unmount(&self) -> Result<(), VfsServerError> {
        self.client
            .unmount(self.handle)
            .await
            .map_call_err("Failed to unmount")
    }

    /// Get the ID of the mount point.
    pub fn id(&self) -> MountId {
        self.id
    }

    /// Get the canonical path of the mount point.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Increment the link count of the mount point.
    pub fn link(&self) {
        self.link_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement the link count of the mount point.
    pub fn unlink(&self) {
        self.link_count.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get the root vnode of the mount point, which is used to access the root directory of the file system.
    pub fn root(&self) -> VNode {
        VNode::new(self.id, self.root)
    }

    /// Lookup a node in the mounted file system by its node ID, returning a vnode that can be used to access the node.
    pub async fn lookup(&self, parent: NodeId, name: &str) -> Result<NodeId, VfsServerError> {
        self.client
            .lookup(self.handle, parent, name)
            .await
            .map_call_err("Failed to lookup")
    }

    /// Create a new node in the mounted file system, returning a vnode that can be used to access the node.
    pub async fn create(
        &self,
        parent: NodeId,
        name: &str,
        r#type: NodeType,
        permissions: Permissions,
    ) -> Result<NodeId, VfsServerError> {
        self.client
            .create(self.handle, parent, name, r#type, permissions)
            .await
            .map_call_err("Failed to create node")
    }

    /// Remove a node from the mounted file system by its node ID and name.
    pub async fn remove(&self, parent: NodeId, name: &str) -> Result<(), VfsServerError> {
        self.client
            .remove(self.handle, parent, name)
            .await
            .map_call_err("Failed to remove node")
    }

    /// Move a node from one location to another in the mounted file system by their parent node IDs and names.
    pub async fn r#move(
        &self,
        src_parent: NodeId,
        src_name: &str,
        dst_parent: NodeId,
        dst_name: &str,
    ) -> Result<(), VfsServerError> {
        self.client
            .r#move(self.handle, src_parent, src_name, dst_parent, dst_name)
            .await
            .map_call_err("Failed to move node")
    }

    /// Get the metadata of a node in the mounted file system.
    pub async fn get_metadata(&self, node: NodeId) -> Result<Metadata, VfsServerError> {
        self.client
            .get_metadata(self.handle, node)
            .await
            .map_call_err("Failed to get metadata")
    }

    /// Set the metadata of a node in the mounted file system.
    pub async fn set_metadata(
        &self,
        node: NodeId,
        permissions: Option<Permissions>,
        size: Option<usize>,
        created: Option<u64>,
        modified: Option<u64>,
    ) -> Result<(), VfsServerError> {
        self.client
            .set_metadata(self.handle, node, permissions, size, created, modified)
            .await
            .map_call_err("Failed to set metadata")
    }

    /// Open a file in the mounted file system by its node ID and return a handle that can be used to access the file.
    pub async fn open_file(
        &self,
        node_id: NodeId,
        open_permissions: Permissions,
    ) -> Result<Handle, VfsServerError> {
        self.client
            .open_file(self.handle, node_id, open_permissions)
            .await
            .map_call_err("Failed to open file")
    }

    /// Close a file in the mounted file system by its handle.
    pub async fn close_file(&self, handle: Handle) -> Result<(), VfsServerError> {
        self.client
            .close_file(self.handle, handle)
            .await
            .map_call_err("Failed to close file")
    }

    /// Read data from a file in the mounted file system by its handle.
    pub async fn read_file(
        &self,
        handle: Handle,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, VfsServerError> {
        self.client
            .read_file(self.handle, handle, offset, buffer)
            .await
            .map_call_err("Failed to read file")
    }

    /// Write data to a file in the mounted file system by its handle.
    pub async fn write_file(
        &self,
        handle: Handle,
        offset: usize,
        buffer: &[u8],
    ) -> Result<usize, VfsServerError> {
        self.client
            .write_file(self.handle, handle, offset, buffer)
            .await
            .map_call_err("Failed to write file")
    }

    /// Open a directory in the mounted file system by its node ID and return a handle that can be used to access the directory.
    pub async fn open_dir(&self, node_id: NodeId) -> Result<Handle, VfsServerError> {
        self.client
            .open_dir(self.handle, node_id)
            .await
            .map_call_err("Failed to open directory")
    }

    /// Close a directory in the mounted file system by its handle.
    pub async fn close_dir(&self, handle: Handle) -> Result<(), VfsServerError> {
        self.client
            .close_dir(self.handle, handle)
            .await
            .map_call_err("Failed to close directory")
    }

    /// List the entries in a directory in the mounted file system by its handle.
    pub async fn list_dir(&self, handle: Handle) -> Result<Vec<DirectoryEntry>, VfsServerError> {
        self.client
            .list_dir(self.handle, handle)
            .await
            .map_call_err("Failed to list directory")
    }

    /// Create a symbolic link in the mounted file system by its parent node ID, name, and target path.
    pub async fn create_symlink(
        &self,
        parent: NodeId,
        name: &str,
        target: &str,
    ) -> Result<NodeId, VfsServerError> {
        self.client
            .create_symlink(self.handle, parent, name, target)
            .await
            .map_call_err("Failed to create symbolic link")
    }

    /// Read the target path of a symbolic link in the mounted file system by its node ID.
    pub async fn read_symlink(&self, node_id: NodeId) -> Result<String, VfsServerError> {
        self.client
            .read_symlink(self.handle, node_id)
            .await
            .map_call_err("Failed to read symbolic link")
    }
}

trait ResultFsCallExt<T> {
    fn map_call_err(self, msg: &'static str) -> Result<T, VfsServerError>;
}

impl<T> ResultFsCallExt<T> for Result<T, FsServerCallError> {
    fn map_call_err(self, msg: &'static str) -> Result<T, VfsServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);

            match e {
                CallError::KernelError(_) => VfsServerError::RuntimeError,

                CallError::ReplyError(FsServerError::InvalidArgument) => {
                    VfsServerError::InvalidArgument
                }
                CallError::ReplyError(FsServerError::RuntimeError) => VfsServerError::RuntimeError,
                CallError::ReplyError(FsServerError::BufferTooSmall) => {
                    VfsServerError::BufferTooSmall
                }
                CallError::ReplyError(FsServerError::NodeNotFound) => VfsServerError::NotFound,
                CallError::ReplyError(FsServerError::NodeAlreadyExists) => {
                    VfsServerError::AlreadyExists
                }
                CallError::ReplyError(FsServerError::NodeBadType) => VfsServerError::BadType,
            }
        })
    }
}
