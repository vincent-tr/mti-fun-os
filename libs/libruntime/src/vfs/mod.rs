mod api;
pub mod fs;
pub mod iface;
pub mod types;

// client API
pub use api::{
    list_mounts, mount, r#move, remove, stat, unmount, Directory, File, Symlink, VfsObject,
};
pub use iface::{DirectoryEntry, MountInfo, VfsServerCallError};
pub use types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions};
