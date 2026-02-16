use core::fmt;

use crate::ipc::{buffer_messages::Buffer, Handle};

use crate::vfs::types::{Metadata, NodeId, NodeType, Permissions};

/// Version of the fs management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in fs management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    Lookup,
    Create,
    Remove,
    Move,
    GetMetadata,
    SetMetadata,

    OpenFile,
    CloseFile,
    ReadFile,
    WriteFile,

    OpenDir,
    CloseDir,
    ListDir,

    CreateSymlink,
    ReadSymlink,

    Mount,
    Unmount,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}

/// Errors used by the fs server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum FsServerError {
    InvalidArgument = 1,
    RuntimeError,
    BufferTooSmall,
}

impl fmt::Display for FsServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
            Self::BufferTooSmall => write!(f, "BufferTooSmall"),
        }
    }
}

/// Parameters for the Lookup message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LookupQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The parent directory of the node to lookup.
    pub parent: NodeId,

    /// The name of the node to lookup.
    pub name: Buffer,
}

impl LookupQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 0;
}

/// Reply of the Lookup message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LookupReply {
    /// The id of the node found by the lookup.
    pub node_id: NodeId,
}

/// Parameters for the Create message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The parent directory of the node to create.
    pub parent: NodeId,

    /// The name of the node to create.
    pub name: Buffer,

    /// The type of the node to create (file or directory).
    pub r#type: NodeType,

    /// Permissions to set to the file or directory.
    pub permissions: Permissions,
}

impl CreateQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 0;
}

/// Reply of the Create message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateReply {
    /// The id of the node created by the create message.
    pub node_id: NodeId,
}

/// Parameters for the Remove message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RemoveQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The parent directory of the node to remove.
    pub parent: NodeId,

    /// The name of the node to remove.
    pub name: Buffer,
}

impl RemoveQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 0;
}

/// Reply of the Remove message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RemoveReply {}

/// Parameters for the Move message.
///
/// Note: can only move within the same filesystem.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MoveQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The parent directory of the node to move.
    pub src_parent: NodeId,

    /// The name of the node to move.
    pub src_name: Buffer,

    /// The new parent directory of the node to move.
    pub dst_parent: NodeId,

    /// The new name of the node.
    pub dst_name: Buffer,
}

impl MoveQueryParameters {
    pub const HANDLE_SRC_NAME_MOBJ: usize = 0;
    pub const HANDLE_DST_NAME_MOBJ: usize = 1;
}

/// Reply of the Move message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MoveReply {}

/// Parameters for the GetMetadata message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetMetadataQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The id of the node to get metadata of.
    pub node_id: NodeId,
}

/// Reply of the GetMetadata message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetMetadataReply {
    /// The metadata of the node.
    pub metadata: Metadata,
}

/// Parameters for the SetMetadata message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetMetadataQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The id of the node to set metadata of.
    pub node_id: NodeId,

    /// The new permissions of the node. If None, the permissions will not be changed.
    pub permissions: Option<Permissions>,

    /// The new size of the node. If None, the size will not be changed.
    pub size: Option<usize>,

    /// The new created time of the node, in milliseconds since the Unix epoch. If None, the created time will not be changed.
    pub created: Option<u64>,

    /// The new modified time of the node, in milliseconds since the Unix epoch. If None, the modified time will not be changed.
    pub modified: Option<u64>,
}

/// Reply of the SetMetadata message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetMetadataReply {}

/// Parameters for the OpenFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenFileQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The id of the node to open.
    pub node_id: NodeId,

    /// Permissions to open the file with.
    pub open_permissions: Permissions,
}

/// Reply of the OpenFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenFileReply {
    /// The handle to the opened file or directory.
    pub handle: Handle,
}

/// Parameters for the CloseFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseFileQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The handle to the opened file to close.
    pub handle: Handle,
}

/// Reply of the CloseFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseFileReply {}

/// Parameters for the ReadFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadFileQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The handle to the opened file to read from.
    pub handle: Handle,

    /// The buffer to read the file content into.
    pub buffer: Buffer,

    /// The offset in the file to read from.
    pub offset: usize,
}

impl ReadFileQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply of the ReadFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadFileReply {
    /// The number of bytes read from the file.
    pub bytes_read: usize,
}

/// Parameters for the WriteFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteFileQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The handle to the opened file to write to.
    pub handle: Handle,

    /// The buffer containing the file content to write.
    pub buffer: Buffer,

    /// The offset in the file to write to.
    pub offset: usize,
}

impl WriteFileQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply of the WriteFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteFileReply {
    /// The number of bytes written to the file.
    pub bytes_written: usize,
}

/// Parameters for the OpenDir message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenDirQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The id of the directory to open.
    pub node_id: NodeId,
}

/// Reply of the OpenDir message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenDirReply {
    /// The handle to the opened directory.
    pub handle: Handle,
}

/// Parameters for the CloseDir message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseDirQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The handle to the opened directory to close.
    pub handle: Handle,
}

/// Reply of the CloseDir message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseDirReply {}

/// Parameters for the ListDir message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListDirQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The handle to the opened directory to list.
    pub handle: Handle,

    /// The buffer to write the directory entries into.
    pub buffer: Buffer,
}

impl ListDirQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply of the ListDir message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListDirReply {
    /// The number of bytes written to the buffer.
    pub buffer_used_len: usize,
}

/// Parameters for the CreateSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateSymlinkQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The parent directory of the symlink to create.
    pub parent: NodeId,

    /// The name of the symlink to create.
    pub name: Buffer,

    /// The target path of the symlink to create.
    pub target: Buffer,
}

impl CreateSymlinkQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 0;
    pub const HANDLE_TARGET_MOBJ: usize = 1;
}

/// Reply of the CreateSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateSymlinkReply {
    /// The id of the symlink created by the create symlink message.
    pub node_id: NodeId,
}

/// Parameters for the ReadSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadSymlinkQueryParameters {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The id of the symlink to read.
    pub node_id: NodeId,

    /// The buffer to write the target path of the symlink into.
    pub buffer: Buffer,
}

impl ReadSymlinkQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 0;
}

/// Reply of the ReadSymlink message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadSymlinkReply {
    /// The number of bytes written to the buffer.
    pub target_len: usize,
}

/// Parameters for the Mount message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MountQueryParameters {
    /// Mount args
    pub args: Buffer,
}

impl MountQueryParameters {
    pub const HANDLE_ARGS_MOBJ: usize = 0;
}

/// Reply of the Mount message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MountReply {
    /// The handle to the mounted filesystem.
    pub mount_handle: Handle,

    /// The id of the root node of the mounted filesystem.
    pub root_node_id: NodeId,
}

/// Parameters for the Unmount message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UnmountQueryParameters {
    /// The handle to the mounted filesystem to unmount.
    pub mount_handle: Handle,
}

/// Reply of the Unmount message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UnmountReply {}
