mod api;
pub mod fs;
pub mod types;
pub mod vfs;

// client API
pub use api::{
    Directory, File, Symlink, VfsObject, list_mounts, mount, r#move, remove, stat, unmount,
};
pub use types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions};
pub use vfs::iface::{DirectoryEntry, MountInfo, VfsServerCallError};
