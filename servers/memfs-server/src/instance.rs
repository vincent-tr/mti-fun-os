use core::{
    cmp::min,
    sync::atomic::{AtomicU64, Ordering},
};

use alloc::{string::String, vec::Vec};
use hashbrown::HashMap;
use libruntime::{
    ipc::Handle,
    vfs::{
        fs::iface::FsServerError,
        iface::DirectoryEntry,
        types::{HandlePermissions, Metadata, NodeId, NodeType, Permissions},
    },
};

use crate::state::State;

/// Instance of a file system, representing a mounted file system with its own state and operations.
#[derive(Debug)]
pub struct FsInstance {
    /// Nodes in the file system, indexed by their unique NodeId.
    nodes: HashMap<NodeId, Node>,

    /// Root node of the file system, which is always a directory and has NodeId 0.
    root: Option<NodeId>,

    /// Mapping of open file handles to their corresponding NodeIds, allowing the file system to track which nodes are currently opened by clients.
    opened_nodes: HashMap<Handle, NodeId>,

    /// Atomic counter for generating unique NodeIds for new nodes created in the file system. This ensures that each node has a distinct identifier.
    id_generator: AtomicU64,
}

impl FsInstance {
    /// Creates a new instance of the file system with an empty root directory.
    pub fn new() -> Self {
        let mut instance = Self {
            nodes: HashMap::new(),
            root: None,
            opened_nodes: HashMap::new(),
            id_generator: AtomicU64::new(0),
        };

        let root = instance.new_node(
            NodeKind::new_directory(),
            Permissions::READ | Permissions::EXECUTE | Permissions::WRITE,
        );
        instance.root = Some(root);

        instance
    }

    /// Retrieves the NodeId of the root directory of the file system.
    pub fn get_root(&self) -> NodeId {
        self.root.expect("Root node should always be initialized")
    }

    /// Looks up a child node by name under the specified parent directory node.
    pub fn lookup(&self, parent: NodeId, name: &str) -> Result<NodeId, FsServerError> {
        let entries = self.get_parent_entries(parent)?;
        entries
            .get(name)
            .copied()
            .ok_or(FsServerError::NodeNotFound)
    }

    /// Creates a new node (file, directory, or symbolic link) under the specified parent directory node with the given name and permissions.
    pub fn create(
        &mut self,
        parent: NodeId,
        name: &str,
        node_type: NodeType,
        permissions: Permissions,
    ) -> Result<NodeId, FsServerError> {
        if name.is_empty() {
            return Err(FsServerError::InvalidArgument);
        }

        if self.get_parent_entries(parent)?.contains_key(name) {
            return Err(FsServerError::NodeAlreadyExists);
        }

        let new_node_id = self.new_node(NodeKind::new(node_type), permissions);

        self.get_parent_entries_mut(parent)
            .expect("Could not get parent")
            .insert(String::from(name), new_node_id);
        self.node_updated(parent);

        Ok(new_node_id)
    }

    /// Removes a child node by name from the specified parent directory node.
    pub fn remove(&mut self, parent: NodeId, name: &str) -> Result<(), FsServerError> {
        let entries = self.get_parent_entries_mut(parent)?;

        let node_id = entries.remove(name).ok_or(FsServerError::NodeNotFound)?;

        self.unlink_node(node_id);
        self.node_updated(parent);

        Ok(())
    }

    /// Moves a node from one parent directory to another, optionally renaming it in the process.
    pub fn r#move(
        &mut self,
        old_parent: NodeId,
        old_name: &str,
        new_parent: NodeId,
        new_name: &str,
    ) -> Result<(), FsServerError> {
        if old_name.is_empty() || new_name.is_empty() {
            return Err(FsServerError::InvalidArgument);
        }

        if self.get_parent_entries(new_parent)?.contains_key(new_name) {
            return Err(FsServerError::NodeAlreadyExists);
        }

        let node_id = self
            .get_parent_entries_mut(old_parent)?
            .remove(old_name)
            .ok_or(FsServerError::NodeNotFound)?;

        self.get_parent_entries_mut(new_parent)
            .expect("Could not get parent")
            .insert(String::from(new_name), node_id);

        self.node_updated(old_parent);
        self.node_updated(new_parent);

        Ok(())
    }

    /// Retrieves the metadata of a node by its NodeId
    pub fn get_metadata(&self, node_id: NodeId) -> Result<Metadata, FsServerError> {
        let node = self
            .nodes
            .get(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        Ok(Metadata {
            r#type: node.kind.r#type(),
            permissions: node.perms,
            size: node
                .kind
                .get_file_data()
                .map(|data| data.len())
                .unwrap_or(0),
            created: node.created,
            modified: node.modified,
        })
    }

    /// Updates the metadata of a node by its NodeId, allowing changes to permissions, size (for files), and timestamps.
    pub fn set_metadata(
        &mut self,
        node_id: NodeId,
        permissions: Option<Permissions>,
        size: Option<usize>,
        created: Option<u64>,
        modified: Option<u64>,
    ) -> Result<(), FsServerError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        if let Some(perms) = permissions {
            node.perms = perms;
        }

        if let Some(size) = size {
            if let Some(data) = node.kind.get_file_data_mut() {
                data.resize(size, 0);
            } else {
                return Err(FsServerError::InvalidArgument);
            }
        }

        if let Some(created) = created {
            node.created = created;
        }

        if let Some(modified) = modified {
            node.modified = modified;
        }

        Ok(())
    }

    /// Opens a file node by its NodeId and returns a handle that can be used for subsequent read/write operations.
    pub fn open_file(
        &mut self,
        node_id: NodeId,
        _open_permissions: HandlePermissions,
    ) -> Result<Handle, FsServerError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        if !node.kind.is_file() {
            return Err(FsServerError::NodeBadType);
        }

        let handle = Self::new_handle();
        self.opened_nodes.insert(handle, node_id);

        self.link_node(node_id);

        Ok(handle)
    }

    /// Closes an open file handle, removing it from the tracking map of opened nodes.
    pub fn close_file(&mut self, handle: Handle) -> Result<(), FsServerError> {
        let node_id = self
            .opened_nodes
            .remove(&handle)
            .ok_or(FsServerError::InvalidArgument)?;

        self.unlink_node(node_id);

        Ok(())
    }

    /// Reads data from an open file handle into the provided buffer, starting at the specified offset. Returns the number of bytes read.
    pub fn read_file(
        &self,
        handle: Handle,
        buffer: &mut [u8],
        offset: usize,
    ) -> Result<usize, FsServerError> {
        let node_id = self
            .opened_nodes
            .get(&handle)
            .copied()
            .ok_or(FsServerError::InvalidArgument)?;

        let node = self
            .nodes
            .get(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        let data = node
            .kind
            .get_file_data()
            .ok_or(FsServerError::NodeBadType)?;

        if offset >= data.len() {
            return Err(FsServerError::InvalidArgument);
        }

        let bytes_to_read = min(buffer.len(), data.len() - offset);
        buffer[..bytes_to_read].copy_from_slice(&data[offset..offset + bytes_to_read]);

        Ok(bytes_to_read)
    }

    /// Writes data from the provided buffer to an open file handle, starting at the specified offset. Returns the number of bytes written.
    pub fn write_file(
        &mut self,
        handle: Handle,
        buffer: &[u8],
        offset: usize,
    ) -> Result<usize, FsServerError> {
        let node_id = self
            .opened_nodes
            .get(&handle)
            .copied()
            .ok_or(FsServerError::InvalidArgument)?;

        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        let data = node
            .kind
            .get_file_data_mut()
            .ok_or(FsServerError::NodeBadType)?;

        if offset >= data.len() {
            return Err(FsServerError::InvalidArgument);
        }

        let bytes_to_write = min(buffer.len(), data.len() - offset);
        data[offset..offset + bytes_to_write].copy_from_slice(&buffer[..bytes_to_write]);
        self.node_updated(node_id);

        Ok(bytes_to_write)
    }

    /// Opens a directory node by its NodeId and returns a handle that can be used for subsequent read operations to list its entries.
    pub fn open_dir(&mut self, node_id: NodeId) -> Result<Handle, FsServerError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        if !node.kind.is_directory() {
            return Err(FsServerError::NodeBadType);
        }

        let handle = Self::new_handle();
        self.opened_nodes.insert(handle, node_id);

        self.link_node(node_id);

        Ok(handle)
    }

    /// Closes an open directory handle, removing it from the tracking map of opened nodes.
    pub fn close_dir(&mut self, handle: Handle) -> Result<(), FsServerError> {
        let node_id = self
            .opened_nodes
            .remove(&handle)
            .ok_or(FsServerError::InvalidArgument)?;

        self.unlink_node(node_id);

        Ok(())
    }

    /// Lists the entries of an open directory handle, returning a vector of (name, NodeId) pairs for each entry in the directory.
    pub fn list_dir(&self, handle: Handle) -> Result<Vec<DirectoryEntry>, FsServerError> {
        let node_id = self
            .opened_nodes
            .get(&handle)
            .copied()
            .ok_or(FsServerError::InvalidArgument)?;

        let node = self
            .nodes
            .get(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        let entries = node
            .kind
            .get_directory_entries()
            .ok_or(FsServerError::NodeBadType)?;

        let dentries = entries
            .iter()
            .map(|(name, &node_id)| DirectoryEntry {
                name: name.clone(),
                r#type: self
                    .nodes
                    .get(&node_id)
                    .expect("Node should exist")
                    .kind
                    .r#type(),
            })
            .collect();

        Ok(dentries)
    }

    /// Creates a new symbolic link node under the specified parent directory node with the given name, target path, and permissions.
    pub fn create_symlink(
        &mut self,
        parent: NodeId,
        name: &str,
        target: &str,
    ) -> Result<NodeId, FsServerError> {
        if name.is_empty() {
            return Err(FsServerError::InvalidArgument);
        }

        if self.get_parent_entries(parent)?.contains_key(name) {
            return Err(FsServerError::NodeAlreadyExists);
        }

        // Note: symlink permissions are ignored.
        let new_node_id = self.new_node(
            NodeKind::new_symlink(String::from(target)),
            Permissions::READ | Permissions::EXECUTE | Permissions::WRITE,
        );

        self.get_parent_entries_mut(parent)
            .expect("Could not get parent")
            .insert(String::from(name), new_node_id);
        self.node_updated(parent);

        Ok(new_node_id)
    }

    pub fn read_symlink(&self, node_id: NodeId) -> Result<String, FsServerError> {
        let node = self
            .nodes
            .get(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        let target = node
            .kind
            .get_symlink_target()
            .ok_or(FsServerError::NodeBadType)?;

        Ok(String::from(target))
    }

    fn new_node(&mut self, kind: NodeKind, perms: Permissions) -> NodeId {
        let id = self.id_generator.fetch_add(1, Ordering::SeqCst);
        let node_id = NodeId::from(id);
        let now = Self::now();

        self.nodes.insert(
            node_id,
            Node {
                kind,
                perms,
                link_count: 1,
                created: now,
                modified: now,
            },
        );

        node_id
    }

    fn link_node(&mut self, node_id: NodeId) {
        let node = self.nodes.get_mut(&node_id).expect("Node should exist");
        node.link_count += 1;
    }

    fn unlink_node(&mut self, node_id: NodeId) {
        let node = self.nodes.get_mut(&node_id).expect("Node should exist");

        assert!(node.link_count > 0, "Link count should never be negative");
        node.link_count -= 1;

        if node.link_count > 0 {
            return;
        }

        let removed_node = self.nodes.remove(&node_id).expect("Failed to remove node");

        // if that's a directory, collect all its children and unlink them as well
        if let Some(entries) = removed_node.kind.get_directory_entries() {
            for (_, &child_id) in entries {
                self.unlink_node(child_id);
            }
        }
    }

    fn node_updated(&mut self, node_id: NodeId) {
        let node = self.nodes.get_mut(&node_id).expect("Node not found");
        node.modified = Self::now();
    }

    fn get_parent_entries(
        &self,
        node_id: NodeId,
    ) -> Result<&HashMap<String, NodeId>, FsServerError> {
        let parent_node = self
            .nodes
            .get(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        parent_node
            .kind
            .get_directory_entries()
            .ok_or(FsServerError::NodeBadType)
    }

    fn get_parent_entries_mut(
        &mut self,
        node_id: NodeId,
    ) -> Result<&mut HashMap<String, NodeId>, FsServerError> {
        let parent_node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        parent_node
            .kind
            .get_directory_entries_mut()
            .ok_or(FsServerError::NodeBadType)
    }

    fn now() -> u64 {
        // TODO
        0
    }

    fn new_handle() -> Handle {
        State::get().handle_generator().generate()
    }
}

/// Represent a node in the file system, which can be a file, directory, or symbolic link, along with its permissions and link count.
#[derive(Debug)]
struct Node {
    /// Kind of the node (file, directory, or symbolic link) along with its specific data.
    kind: NodeKind,

    /// Permissions of the node, which determine who can read, write, or execute it.
    perms: Permissions,

    /// Number of hard links to this node. If this count drops to zero, the node can be deleted from the file system.
    link_count: usize,

    /// Creation time of the node, in milliseconds since the Unix epoch.
    created: u64,

    /// Last modification time of the node, in milliseconds since the Unix epoch.
    modified: u64,
}

/// Kind of a node in the file system, which can be a file, directory, or symbolic link, along with its specific data.
#[derive(Debug)]
enum NodeKind {
    File { data: Vec<u8> },
    Directory { entries: HashMap<String, NodeId> },
    Symlink { target: String },
}

impl NodeKind {
    /// Creates a new NodeKind based on the given NodeType.
    pub fn new(r#type: NodeType) -> Self {
        match r#type {
            NodeType::File => Self::new_file(),
            NodeType::Directory => Self::new_directory(),
            NodeType::Symlink => panic!("Symlink creation requires a target path"),
        }
    }

    /// Returns the NodeType corresponding to this NodeKind.
    pub fn r#type(&self) -> NodeType {
        match self {
            NodeKind::File { .. } => NodeType::File,
            NodeKind::Directory { .. } => NodeType::Directory,
            NodeKind::Symlink { .. } => NodeType::Symlink,
        }
    }

    /// Creates a new directory node with the given entries.
    pub fn new_file() -> Self {
        NodeKind::File { data: Vec::new() }
    }

    /// Creates a new directory node with the given entries.
    pub fn new_directory() -> Self {
        NodeKind::Directory {
            entries: HashMap::new(),
        }
    }

    /// Creates a new symbolic link node with the given target path.
    pub fn new_symlink(target: String) -> Self {
        NodeKind::Symlink { target }
    }

    /// Checks if the node is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self, NodeKind::Directory { .. })
    }

    /// Checks if the node is a file.
    pub fn is_file(&self) -> bool {
        matches!(self, NodeKind::File { .. })
    }

    /// If this node is a file, returns a reference to its data. Otherwise, returns `None`.
    pub fn get_file_data(&self) -> Option<&[u8]> {
        if let NodeKind::File { data } = self {
            Some(data)
        } else {
            None
        }
    }

    /// If this node is a file, returns a mutable reference to its data. Otherwise, returns `None`.
    pub fn get_file_data_mut(&mut self) -> Option<&mut Vec<u8>> {
        if let NodeKind::File { data } = self {
            Some(data)
        } else {
            None
        }
    }

    /// If this node is a directory, returns a reference to its entries. Otherwise, returns `None`.
    pub fn get_directory_entries(&self) -> Option<&HashMap<String, NodeId>> {
        if let NodeKind::Directory { entries } = self {
            Some(entries)
        } else {
            None
        }
    }

    /// If this node is a directory, returns a mutable reference to its entries. Otherwise, returns `None`.
    pub fn get_directory_entries_mut(&mut self) -> Option<&mut HashMap<String, NodeId>> {
        if let NodeKind::Directory { entries } = self {
            Some(entries)
        } else {
            None
        }
    }

    /// If this node is a symbolic link, returns its target path. Otherwise, returns `None`.
    pub fn get_symlink_target(&self) -> Option<&str> {
        if let NodeKind::Symlink { target } = self {
            Some(target)
        } else {
            None
        }
    }
}
