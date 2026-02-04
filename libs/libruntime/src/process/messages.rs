use alloc::fmt;

use crate::ipc::{buffer::messages::Buffer, handle::Handle};

/// Name of the IPC port for the process server.
pub const PORT_NAME: &str = "process-server";

/// Version of the process management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in process management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    CreateProcess = 1,
    GetStartupInfo = 2,
    UpdateEnv = 3,
    SetExitCode = 4,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}

/// Errors used by the process server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum ProcessServerError {
    InvalidArgument = 1,
    InvalidBinaryFormat,
    RuntimeError,
}

impl fmt::Display for ProcessServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::InvalidBinaryFormat => write!(f, "InvalidBinaryFormat"),
            Self::RuntimeError => write!(f, "RuntimeError"),
        }
    }
}

/// Parameters for the CreateProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateProcessQueryParameters {
    pub name: Buffer,
    pub binary: Buffer,
}

impl CreateProcessQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
    pub const HANDLE_BINARY_MOBJ: usize = 2;
    pub const HANDLE_ENV_MOBJ: usize = 3;
    pub const HANDLE_ARGS_MOBJ: usize = 4;
}

/// Reply for the CreateProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateProcessReply {
    /// Server handle to the created process
    pub handle: Handle,
}
