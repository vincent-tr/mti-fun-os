use core::fmt;

use crate::ipc::buffer_messages::Buffer;

use super::types::{Metadata, NodeType, Permissions};

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

    Open,
    Read,
    Write,
    Close,

    OpenDir,
    ListDir,
    CloseDir,

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
    /// The parent directory of the node to lookup.
    pub parent: NodeId,

    /// The name of the node to lookup.
    pub name: Buffer,
}

/// Reply of the Lookup message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LookupResult {
    /// The id of the node found by the lookup.
    pub node_id: NodeId,
}

/// Parameters for the Create message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateQueryParameters {
    /// The parent directory of the node to create.
    pub parent: NodeId,

    /// The name of the node to create.
    pub name: Buffer,

    /// The type of the node to create (file or directory).
    pub r#type: NodeType,

    /// Permissions to set to the file or directory.
    pub permissions: Permissions,
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
    /// The parent directory of the node to remove.
    pub parent: NodeId,

    /// The name of the node to remove.
    pub name: Buffer,
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
    /// The parent directory of the node to move.
    pub parent: NodeId,

    /// The name of the node to move.
    pub name: Buffer,

    /// The new name of the node.
    pub new_name: Buffer,
}

/// Reply of the Move message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MoveReply {}

/// Parameters for the GetMetadata message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetMetadataQueryParameters {
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

/// Reply of the SetMetadata message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetMetadataReply {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(u64);
