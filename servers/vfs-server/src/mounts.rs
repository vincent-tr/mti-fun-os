use hashbrown::HashMap;
use libruntime::vfs::fs::iface::{Client, NodeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MountId(u64);

/// Mount table, which contains all the mounted file systems and their mount points.
#[derive(Debug)]
pub struct MountTable {
    /// List of mounted file systems, indexed by their mount IDs.
    mounts: HashMap<MountId, Mount>,

    /// The mount ID of the root file system, which is used to access the root directory.
    root_mount: Option<MountId>,

    /// Mapping from VNode to MountId, which is used to find the mount point of a vnode.
    mountpoints: HashMap<VNode, MountId>,
}

impl MountTable {
    /// Create a new mount table.
    pub fn new() -> Self {
        Self {
            mounts: HashMap::new(),
            root_mount: None,
            mountpoints: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Mount {
    /// The ID of the mount point, which is used to identify the file system in the VNode.
    id: MountId,

    /// The file system implementation of the mount point.
    client: Client<'static>,

    /// The root node of the file system, which is used to access the file system.
    root: NodeId,
}

/// A vnode represents a node in the virtual file system, which is identified by its mount point and node ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VNode {
    /// The mount point of the vnode, which identifies the file system it belongs to.
    pub mount: MountId,

    /// The node ID of the vnode, which identifies the node within the file system.
    pub node: NodeId,
}
