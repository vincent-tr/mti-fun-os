use core::fmt;

use crate::ipc::{buffer_messages::Buffer, Handle};

/// Name of the IPC port for the VFS server.
pub const PORT_NAME: &str = "vfs-server";

/// Version of the VFS management messages.
pub const VERSION: u16 = 1;

use crate::vfs::types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions};

/// Types of messages used in VFS management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    // Messages for handles
    Open = 1,
    Close,
    Stat,
    SetPermissions,

    // Messages for file handles
    Read,
    Write,
    Resize,

    // Messages for directory handles
    List,
    Move,
    Remove,

    // Messages for symlinks handles
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

/// Errors used by the VFS server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum VfsServerError {
    InvalidArgument = 1,
    RuntimeError,
    BufferTooSmall,
    NotFound,
    AlreadyExists,
    BadType,
    Busy,
    TooManySymlinks,
    NotDirectory,
    AccessDenied,
}

impl fmt::Display for VfsServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
            Self::BufferTooSmall => write!(f, "BufferTooSmall"),
            Self::NotFound => write!(f, "NotFound"),
            Self::AlreadyExists => write!(f, "AlreadyExists"),
            Self::BadType => write!(f, "BadType"),
            Self::Busy => write!(f, "Busy"),
            Self::TooManySymlinks => write!(f, "TooManySymlinks"),
            Self::NotDirectory => write!(f, "NotDirectory"),
            Self::AccessDenied => write!(f, "AccessDenied"),
        }
    }
}

/// Parameters for the Open message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenQueryParameters {
    /// Path to the file or directory to open.
    pub path: Buffer,

    /// Type of the node to open (file, directory or symlink).
    /// The server will return an error if the node exists but is of a different type.
    ///
    /// Note: this API can open a symlink, but it cannot create a new one.
    /// Use CreateSymlink to create a new symlink.
    ///
    /// Note: if the type is None, the server cannot create a new node and will return an error if the node does not exist.
    pub r#type: Option<NodeType>,

    /// Mode to open the file or directory with.
    pub mode: OpenMode,

    /// If the last item of the path is a symlink, this flag indicates whether to follow the symlink or not.
    /// If true, the server will open the target of the symlink.
    /// If false, the server will open the symlink itself.
    pub no_follow: bool,

    /// Permissions to set to the file or directory if it is created.
    /// If the file or directory already exists, this field is ignored.
    pub permissions: Permissions,

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
    /// Handle to the opened node.
    pub handle: Handle,
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

/// Parameters for the Stat message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatQueryParameters {
    /// Handle to the file, directory or symlink to stat.
    pub handle: Handle,
}

/// Reply for the Stat message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatReply {
    /// Metadata of the file, directory or symlink.
    pub metadata: Metadata,
}

/// Parameters for the SetPermissions message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetPermissionsQueryParameters {
    /// Handle of the node to set permissions.
    pub handle: Handle,

    /// New permissions for the node.
    pub permissions: Permissions,
}

/// Reply for the SetPermissions message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetPermissionsReply {}

/// Parameters for the Read message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadQueryParameters {
    /// Handle to the file to read from.
    pub handle: Handle,

    /// Buffer to read data into.
    pub buffer: Buffer,

    /// Offset in the file to start reading from.
    ///
    /// Note: the read len is determined by the size of the buffer.
    pub offset: usize,
}

impl ReadQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply for the Read message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadReply {
    /// Number of bytes read.
    pub bytes_read: usize,
}

/// Parameters for the Write message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteQueryParameters {
    /// Handle to the file to write to.
    pub handle: Handle,

    /// Buffer containing the data to write.
    pub buffer: Buffer,

    /// Offset in the file to start writing to.
    ///
    /// Note: the write len is determined by the size of the buffer.
    pub offset: usize,
}

impl WriteQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply for the Write message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteReply {
    /// Number of bytes written.
    pub bytes_written: usize,
}

/// Parameters for the Resize message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResizeQueryParameters {
    /// Handle to the file to resize.
    pub handle: Handle,

    /// New size of the file in bytes.
    pub new_size: usize,
}

/// Reply for the Resize message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResizeReply {}

/// Parameters for the List message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListQueryParameters {
    /// Handle to the directory to list.
    pub handle: Handle,

    /// Buffer to write the list of entries into.
    pub buffer: Buffer,
}

impl ListQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply for the List message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListReply {
    /// Number of bytes used in the buffer to write the list of entries (if the call succeeds)
    pub buffer_used_len: usize,
}

/// Parameters for the Move message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MoveQueryParameters {
    /// Handle of the directory to move the node from.
    pub old_dir: Handle,

    /// Name of the node to move, relative to the old directory.
    pub old_name: Buffer,

    /// Handle of the directory to move the node to.
    pub new_dir: Handle,

    /// Name of the node to move, relative to the new directory.
    pub new_name: Buffer,
}

impl MoveQueryParameters {
    pub const HANDLE_OLD_NAME_MOBJ: usize = 0;
    pub const HANDLE_NEW_NAME_MOBJ: usize = 1;
}

/// Reply for the Move message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MoveReply {}

/// Parameters for the Remove message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RemoveQueryParameters {
    /// Handle of the directory to remove the node from.
    pub dir: Handle,

    /// Name of the node to remove, relative to the directory.
    pub name: Buffer,
}

impl RemoveQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 0;
}

/// Reply for the Remove message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RemoveReply {}

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
pub struct CreateSymlinkReply {
    /// Handle to the create symlink.
    pub handle: Handle,
}

/// Parameters for the ReadSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadSymlinkQueryParameters {
    /// Handle to the symlink to read.
    pub handle: Handle,

    /// Buffer to write the target path into.
    pub buffer: Buffer,
}

impl ReadSymlinkQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
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
pub struct ListMountsReply {
    /// Number of bytes used in the buffer to write the list of mounts (if the call succeeds)
    pub buffer_used_len: usize,
}
