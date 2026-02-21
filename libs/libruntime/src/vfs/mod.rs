mod api;
pub mod fs;
pub mod iface;
pub mod types;

// client API
pub use api::{
    Directory, File, Symlink, VfsObject, list_mounts, mount, r#move, remove, stat, unmount,
};
pub use iface::{DirectoryEntry, MountInfo, VfsServerCallError};
pub use types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions};
