// Reuse the Permissions type from the kobject module, since it is the same as the one used for paging permissions.
pub use crate::kobject::Permissions;

use bitflags::bitflags;

/// Metadata of a Node in the filesystem, used in the Stat messages.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Metadata {
    /// Type of the node (file, directory, or symlink).
    pub r#type: NodeType,

    /// Permissions of the node.
    ///
    /// For symlink, the permissions are ignored and should be set to 0.
    pub permissions: Permissions,

    /// For files, the size of the file in bytes.
    ///
    /// For directories and symlinks, this field is ignored and should be set to 0.
    pub size: usize,

    /// Creation time of the node, in milliseconds since the Unix epoch.
    pub created: u64,

    /// Last modification time of the node, in milliseconds since the Unix epoch.
    pub modified: u64,
}

/// Types of nodes in the filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeType {
    /// A regular file.
    File = 1,

    /// A directory.
    Directory,

    /// A symbolic link.
    Symlink,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum OpenMode {
    /// Open the file or directory if it exists, otherwise return an error.
    OpenExisting = 1,

    /// Open the file or directory if it exists, otherwise create it.
    OpenAlways,

    /// Create a new file or directory, returning an error if it already exists.
    CreateNew,

    /// Create a new file or directory, overwriting it if it already exists.
    CreateAlways,
}

bitflags! {
    /// Possible handle permissions
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct HandlePermissions: u64 {
        /// No access
        const NONE = 0;

        /// Node can be read
        const READ = 1 << 0;

        /// Node can be written
        const WRITE = 1 << 1;
    }
}

/// A unique identifier for a node in the filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl From<u64> for NodeId {
    fn from(value: u64) -> Self {
        NodeId(value)
    }
}
