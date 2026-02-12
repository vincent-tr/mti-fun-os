// Reuse the Permissions type from the kobject module, since it is the same as the one used for paging permissions.
pub use crate::kobject::Permissions;

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
