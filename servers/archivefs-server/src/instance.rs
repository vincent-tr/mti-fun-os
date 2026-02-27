use core::cmp::min;

use alloc::{string::String, vec::Vec};
use hashbrown::HashMap;
use libruntime::{
    ipc::Handle,
    time::DateTime,
    vfs::{
        fs::iface::FsServerError,
        iface::DirectoryEntry,
        types::{HandlePermissions, Metadata, NodeId, NodeType, Permissions},
    },
};
use log::{debug, error};

use crate::{
    archive::{Archive, ArchiveBuffer, ArchiveEntry, ArchiveString},
    state::State,
};

/// Instance of a file system, representing a mounted file system with its own state and operations.
#[derive(Debug)]
pub struct FsInstance {
    /// Nodes in the file system, indexed by their unique NodeId.
    nodes: HashMap<NodeId, Node>,

    /// Root node of the file system, which is always a directory and has NodeId 0.
    root: Option<NodeId>,

    /// Mapping of open file handles to their corresponding NodeIds, allowing the file system to track which nodes are currently opened by clients.
    opened_nodes: HashMap<Handle, NodeId>,
}

impl FsInstance {
    /// Creates a new instance of the file system with an empty root directory.
    pub fn new(args: &[u8]) -> Result<Self, FsServerError> {
        // Copy the buffer into archive
        let archive = Archive::new(args);

        let mut names = HashMap::new();
        let mut inodes = HashMap::new();

        for entry in archive.iter_entries() {
            if names.insert(entry.name().clone(), entry.clone()).is_some() {
                error!("Duplicate file name in archive: '{}'", entry.name());
                return Err(FsServerError::InvalidArgument);
            }

            if inodes.insert(entry.inode(), entry.clone()).is_some() {
                error!(
                    "Duplicate inode in archive: '{}' (hardlinks not supported)",
                    entry.inode()
                );
                return Err(FsServerError::InvalidArgument);
            }

            if !entry.mode().contains(cpio_reader::Mode::DIRECTORY)
                && !entry.mode().contains(cpio_reader::Mode::REGULAR_FILE)
                && !entry.mode().contains(cpio_reader::Mode::SYMBOLIK_LINK)
            {
                error!(
                    "Unsupported file type in archive for '{}': mode=0o{:o}",
                    entry.name(),
                    entry.mode().bits()
                );
                return Err(FsServerError::InvalidArgument);
            }

            debug!(
                "Found file: name='{}', ino={}, mode=0o{:o}, mtime={}",
                entry.name(),
                entry.inode(),
                entry.mode().bits(),
                entry.mtime(),
            );
        }

        let mut instance = Self {
            nodes: HashMap::new(),
            root: None,
            opened_nodes: HashMap::new(),
        };
        /*
                let root = instance.new_node(
                    NodeKind::new_directory(),
                    Permissions::READ | Permissions::EXECUTE | Permissions::WRITE,
                );
                instance.root = Some(root);
        */
        Ok(instance)
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

    /// Retrieves the metadata of a node by its NodeId
    pub fn get_metadata(&self, node_id: NodeId) -> Result<Metadata, FsServerError> {
        let node = self
            .nodes
            .get(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        let created = node.created.into();

        Ok(Metadata {
            r#type: node.r#type(),
            permissions: node.perms,
            size: node.get_file_data().map(|data| data.len()).unwrap_or(0),
            created,
            modified: created, // Since the archive is read-only, we can use the same timestamp for created and modified
        })
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

        if !node.is_file() {
            return Err(FsServerError::NodeBadType);
        }

        let handle = Self::new_handle();
        self.opened_nodes.insert(handle, node_id);

        Ok(handle)
    }

    /// Closes an open file handle, removing it from the tracking map of opened nodes.
    pub fn close_file(&mut self, handle: Handle) -> Result<(), FsServerError> {
        self.opened_nodes
            .remove(&handle)
            .ok_or(FsServerError::InvalidArgument)?;

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

        let data = node.get_file_data().ok_or(FsServerError::NodeBadType)?;

        if offset >= data.len() {
            return Err(FsServerError::InvalidArgument);
        }

        let bytes_to_read = min(buffer.len(), data.len() - offset);
        buffer[..bytes_to_read].copy_from_slice(&data[offset..offset + bytes_to_read]);

        Ok(bytes_to_read)
    }

    /// Opens a directory node by its NodeId and returns a handle that can be used for subsequent read operations to list its entries.
    pub fn open_dir(&mut self, node_id: NodeId) -> Result<Handle, FsServerError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        if !node.is_directory() {
            return Err(FsServerError::NodeBadType);
        }

        let handle = Self::new_handle();
        self.opened_nodes.insert(handle, node_id);

        Ok(handle)
    }

    /// Closes an open directory handle, removing it from the tracking map of opened nodes.
    pub fn close_dir(&mut self, handle: Handle) -> Result<(), FsServerError> {
        self.opened_nodes
            .remove(&handle)
            .ok_or(FsServerError::InvalidArgument)?;

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
            .get_directory_entries()
            .ok_or(FsServerError::NodeBadType)?;

        let dentries = entries
            .iter()
            .map(|(name, &node_id)| DirectoryEntry {
                name: String::from(name),
                r#type: self
                    .nodes
                    .get(&node_id)
                    .expect("Node should exist")
                    .r#type(),
            })
            .collect();

        Ok(dentries)
    }

    pub fn read_symlink(&self, node_id: NodeId) -> Result<String, FsServerError> {
        let node = self
            .nodes
            .get(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        let target = node
            .get_symlink_target()
            .ok_or(FsServerError::NodeBadType)?;

        Ok(String::from(target))
    }

    fn get_parent_entries(
        &self,
        node_id: NodeId,
    ) -> Result<&HashMap<ArchiveString, NodeId>, FsServerError> {
        let parent_node = self
            .nodes
            .get(&node_id)
            .ok_or(FsServerError::NodeNotFound)?;

        parent_node
            .get_directory_entries()
            .ok_or(FsServerError::NodeBadType)
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

    /// Creation time of the node.
    created: DateTime,
}

impl Node {
    /// Creates a new node from the given archive entry, extracting its type, permissions, and content to initialize the node's kind and metadata.
    pub fn new(entry: &ArchiveEntry) -> Result<Self, FsServerError> {
        let mut perms = Permissions::NONE;
        if entry.mode().contains(cpio_reader::Mode::USER_READABLE) {
            perms |= Permissions::READ;
        }
        if entry.mode().contains(cpio_reader::Mode::USER_EXECUTABLE) {
            perms |= Permissions::EXECUTE;
        }
        // Note: ReadOnly archive -> no write permissions

        Ok(Self {
            kind: NodeKind::new(entry)?,
            perms,
            created: entry.mtime(),
        })
    }

    /// Returns the NodeType corresponding to this NodeKind.
    pub fn r#type(&self) -> NodeType {
        match self.kind {
            NodeKind::File { .. } => NodeType::File,
            NodeKind::Directory { .. } => NodeType::Directory,
            NodeKind::Symlink { .. } => NodeType::Symlink,
        }
    }

    /// Checks if the node is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self.kind, NodeKind::Directory { .. })
    }

    /// Checks if the node is a file.
    pub fn is_file(&self) -> bool {
        matches!(self.kind, NodeKind::File { .. })
    }

    /// If this node is a file, returns a reference to its data. Otherwise, returns `None`.
    pub fn get_file_data(&self) -> Option<&[u8]> {
        if let NodeKind::File { data } = &self.kind {
            Some(data.as_slice())
        } else {
            None
        }
    }

    /// If this node is a directory, returns a reference to its entries. Otherwise, returns `None`.
    pub fn get_directory_entries(&self) -> Option<&HashMap<ArchiveString, NodeId>> {
        if let NodeKind::Directory { entries } = &self.kind {
            Some(entries)
        } else {
            None
        }
    }

    /// If this node is a symbolic link, returns its target path. Otherwise, returns `None`.
    pub fn get_symlink_target(&self) -> Option<&str> {
        if let NodeKind::Symlink { target } = &self.kind {
            Some(target.as_str())
        } else {
            None
        }
    }
}

/// Kind of a node in the file system, which can be a file, directory, or symbolic link, along with its specific data.
#[derive(Debug)]
enum NodeKind {
    File {
        data: ArchiveBuffer,
    },
    Directory {
        entries: HashMap<ArchiveString, NodeId>,
    },
    Symlink {
        target: ArchiveString,
    },
}

impl NodeKind {
    pub fn new(entry: &ArchiveEntry) -> Result<Self, FsServerError> {
        if entry.mode().contains(cpio_reader::Mode::DIRECTORY) {
            Ok(NodeKind::Directory {
                entries: HashMap::new(),
            })
        } else if entry.mode().contains(cpio_reader::Mode::REGULAR_FILE) {
            Ok(NodeKind::File {
                data: entry.content().clone(),
            })
        } else if entry.mode().contains(cpio_reader::Mode::SYMBOLIK_LINK) {
            Ok(NodeKind::Symlink {
                target: unsafe { ArchiveString::from_buffer(entry.content().clone()) },
            })
        } else {
            Err(FsServerError::InvalidArgument)
        }
    }
}
