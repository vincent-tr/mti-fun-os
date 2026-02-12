use core::fmt;

use crate::ipc::{buffer_messages::Buffer, Handle};

// Reuse the Permissions type from the kobject module, since it is the same as the one used for paging permissions.
pub use crate::kobject::Permissions;

/// Name of the IPC port for the process server.
pub const PORT_NAME: &str = "vfs-server";

/// Version of the vfs management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in vfs management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    // Messages for File or Directory handles
    Open = 1,
    Create,
    Close,
    Stat,
    SetPermissions,

    // Messages for paths
    Rename, // Note: need nofollow to rename the symlink itself instead of the target
    Remove, // Note: need nofollow to rename the symlink itself instead of the target

    // Messages for file handles
    Read,
    Write,
    Resize,

    // Messages for directory handles
    List, // Handle

    // Messages for symlinks
    CreateSymlink,
    ReadSymlink,

    // Messages for mount points
    Mount,
    Unmount,
    ListMounts,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}

/// Errors used by the vfs server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum VfsServerError {
    InvalidArgument = 1,
    RuntimeError,
    BufferTooSmall,
}

impl fmt::Display for VfsServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
            Self::BufferTooSmall => write!(f, "BufferTooSmall"),
        }
    }
}

/// Parameters for the Open message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenQueryParameters {
    /// Path to the file or directory to open.
    pub path: Buffer,

    /// Handle permissions
    pub handle_permissions: HandlePermissions,
}

impl OpenQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the Open message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenReply {
    /// Handle to the opened file or directory.
    pub handle: Handle,

    /// Type of the opened node (file or directory).
    pub r#type: NodeType,
}

/// Parameters for the Create message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateQueryParameters {
    /// Path to the file or directory to create.
    pub path: Buffer,

    /// Type of the node to create (file or directory).
    pub r#type: NodeType,

    /// Permissions for the new file or directory.
    pub permissions: Permissions,

    /// Whether to overwrite the node if it already exists. If false, the server will return an error if the node already exists.
    pub overwrite: bool,

    /// Handle permissions
    pub handle_permissions: HandlePermissions,
}

impl CreateQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the Create message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateReply {
    /// Handle to the opened file or directory.
    pub handle: Handle,

    /// Type of the opened node (file or directory).
    pub r#type: NodeType,
}

/// Parameters for the Close message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseQueryParameters {
    /// Handle to close.
    pub handle: Handle,
}

/// Reply for the Close message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseReply {}

/// Parameters for the StatPath message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatPathQueryParameters {
    /// Path to the node to stat.
    pub path: Buffer,
}

impl StatPathQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the StatPath message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatPathReply {
    pub metadata: Metadata,
}

/// Parameters for the StatHandle message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatHandleQueryParameters {
    /// Handle to the node to stat.
    pub handle: Handle,
}

/// Reply for the StatHandle message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatHandleReply {
    pub metadata: Metadata,
}

/// Parameters for the Rename message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RenameQueryParameters {
    /// Path to the node to rename.
    pub old_path: Buffer,

    /// New path for the node.
    pub new_path: Buffer,
}

impl RenameQueryParameters {
    pub const HANDLE_OLD_PATH_MOBJ: usize = 0;
    pub const HANDLE_NEW_PATH_MOBJ: usize = 1;
}

/// Reply for the Rename message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RenameReply {}

/// Parameters for the Remove message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RemoveQueryParameters {
    /// Path to the node to remove.
    pub path: Buffer,
}

impl RemoveQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the Remove message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RemoveReply {}

/// Parameters for the SetPermissions message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetPermissionsQueryParameters {
    /// Path to the node to set permissions for.
    pub path: Buffer,

    /// New permissions for the node.
    pub permissions: Permissions,
}

impl SetPermissionsQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the SetPermissions message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetPermissionsReply {}

/// Parameters for the CreateFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateFileQueryParameters {
    /// Path to the file to create.
    pub path: Buffer,

    /// Permissions for the new file.
    pub permissions: Permissions,
}

impl CreateFileQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the CreateFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateFileReply {
    /// Handle to the newly created file.
    pub handle: Handle,
}

/// Parameters for the OpenFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenFileQueryParameters {
    /// Path to the file to open.
    pub path: Buffer,
}

impl OpenFileQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the OpenFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenFileReply {
    /// Handle to the opened file.
    pub handle: Handle,
}

/// Parameters for the ReadFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadFileQueryParameters {
    /// Handle to the file to read from.
    pub handle: Handle,

    /// Buffer to read data into.
    pub buffer: Buffer,

    /// Offset in the file to start reading from.
    pub offset: u64,
}

impl ReadFileQueryParameters {
    pub const HANDLE_FILE_MOBJ: usize = 0;
    pub const HANDLE_BUFFER_MOBJ: usize = 1;
}

/// Reply for the ReadFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadFileReply {
    /// Number of bytes read.
    pub bytes_read: u64,
}

/// Parameters for the WriteFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteFileQueryParameters {
    /// Handle to the file to write to.
    pub handle: Handle,

    /// Buffer containing the data to write.
    pub buffer: Buffer,

    /// Offset in the file to start writing to.
    pub offset: u64,
}

impl WriteFileQueryParameters {
    pub const HANDLE_FILE_MOBJ: usize = 0;
    pub const HANDLE_BUFFER_MOBJ: usize = 1;
}

/// Reply for the WriteFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteFileReply {
    /// Number of bytes written.
    pub bytes_written: u64,
}

/// Parameters for the ResizeFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResizeFileQueryParameters {
    /// Handle to the file to resize.
    pub handle: Handle,

    /// New size of the file in bytes.
    pub new_size: u64,
}

/// Reply for the ResizeFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResizeFileReply {}

/// Parameters for the CreateDirectory message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateDirectoryQueryParameters {
    /// Path to the directory to create.
    pub path: Buffer,

    /// Permissions for the new directory.
    pub permissions: Permissions,
}

impl CreateDirectoryQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the CreateDirectory message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateDirectoryReply {}

/// Parameters for the OpenDirectory message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenDirectoryQueryParameters {
    /// Path to the directory to open.
    pub path: Buffer,
}

impl OpenDirectoryQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
}

/// Reply for the OpenDirectory message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenDirectoryReply {
    /// Handle to the opened directory.
    pub handle: Handle,
}

/// Parameters for the ListDirectory message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListDirectoryQueryParameters {
    /// Handle to the directory to list.
    pub handle: Handle,

    /// Buffer to write the list of entries into.
    pub buffer: Buffer,
}

impl ListDirectoryQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply for the ListDirectory message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListDirectoryReply {}

/// Parameters for the CreateSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateSymlinkQueryParameters {
    /// Path to the symlink to create.
    pub path: Buffer,

    /// Path that the symlink points to.
    pub target: Buffer,
}

impl CreateSymlinkQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
    pub const HANDLE_TARGET_MOBJ: usize = 1;
}

/// Reply for the CreateSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateSymlinkReply {}

/// Parameters for the ReadSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadSymlinkQueryParameters {
    /// Path to the symlink to read.
    pub path: Buffer,

    /// Buffer to write the target path into.
    pub buffer: Buffer,
}

impl ReadSymlinkQueryParameters {
    pub const HANDLE_PATH_MOBJ: usize = 0;
    pub const HANDLE_BUFFER_MOBJ: usize = 1;
}

/// Reply for the ReadSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadSymlinkReply {
    /// Length of the target path.
    pub target_length: usize,
}

/// Parameters for the Mount message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MountQueryParameters {
    /// Path to the mount point.
    pub mount_point: Buffer,

    /// Port name of the filesystem driver.
    pub fs_port_name: Buffer,

    /// Mount args, to pass to the filesystem driver.
    ///
    /// The content of this buffer is opaque to the vfs server and is only passed to the filesystem driver.
    pub args: Buffer,
}

impl MountQueryParameters {
    pub const HANDLE_MOUNT_POINT_MOBJ: usize = 0;
    pub const HANDLE_FS_PORT_NAME_MOBJ: usize = 1;
    pub const HANDLE_ARGS_MOBJ: usize = 2;
}

/// Reply for the Mount message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MountReply {}

/// Parameters for the Unmount message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UnmountQueryParameters {
    /// Path to the mount point to unmount.
    pub mount_point: Buffer,
}

impl UnmountQueryParameters {
    pub const HANDLE_MOUNT_POINT_MOBJ: usize = 0;
}

/// Reply for the Unmount message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UnmountReply {}

/// Parameters for the ListMounts message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListMountsQueryParameters {
    /// Buffer to write the list of mounts into.
    pub buffer: Buffer,
}

impl ListMountsQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply for the ListMounts message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListMountsReply {}

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
    pub size: u64,
}

/// Types of nodes in the filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeType {
    /// A regular file.
    File,

    /// A directory.
    Directory,

    /// A symbolic link.
    Symlink,
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
