use core::fmt;

use crate::ipc::{ Handle, buffer_messages::Buffer };


/// Name of the IPC port for the process server.
pub const PORT_NAME: &str = "vfs-server";

/// Version of the vfs management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in vfs management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
  // Message for all types of handles
  Close = 1,
  StatPath,
  StatHandle,
  Rename,
  Remove,
  SetPermissionsPath,
  SetPermissionsHandle,

  // Messages for file handles
  CreateFile,
  OpenFile,
  ReadFile,
  WriteFile,
  ResizeFile,

  // Messages for directory handles
  CreateDirectory,
  OpenDirectory,
  ListDirectory,

  // Messages for symlinks
  CreateSymlink,
  ReadSymlink,

  // Messages for mount points
  Mount,
  Unmount,
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

/// Parameters for the Close message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseQueryParameters {
    /// Path to the file or directory or symlink to stat.
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
    /// Path to the file or directory or symlink to stat.
    pub path: Buffer,
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
    /// Handle to the file or directory or symlink to stat.
    pub handle: Handle,
}

/// Reply for the StatHandle message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatHandleReply {
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Metadata {
    // TODO
}

//////////////

/// Parameters for the OpenFile message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenFileMessageQueryParameters {
    /// Path to the file or directory or symlink to open.
    pub path: Buffer,
}
