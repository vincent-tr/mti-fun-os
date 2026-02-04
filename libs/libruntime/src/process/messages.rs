use alloc::fmt;

use crate::ipc::{buffer_messages::Buffer, Handle};

/// Name of the IPC port for the process server.
pub const PORT_NAME: &str = "process-server";

/// Version of the process management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in process management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    /// Messages for self process management
    GetStartupInfo = 1,
    UpdateName,
    UpdateEnv,
    SetExitCode,

    /// Messages for managing other processes
    CreateProcess,
    OpenProcess,
    ListProcesses,
    CloseProcess,
    TerminateProcess,
    GetProcessName,
    GetProcessEnv,
    GetProcessArgs,
    GetProcessExitCode,
    RegisterProcessExitNotification,
    UnregisterProcessExitNotification,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}

/// Types of notifications sent by the process server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NotificationType {
    ProcessExit = 1,
}

impl From<NotificationType> for u16 {
    fn from(value: NotificationType) -> Self {
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

/// Parameters for the GetStartupInfo message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetStartupInfoQueryParameters {}

/// Reply for the GetStartupInfo message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetStartupInfoReply {
    pub name: Buffer,
}

impl GetStartupInfoReply {
    pub const HANDLE_NAME_MOBJ: usize = 0; // Ownership transferred to the client
    pub const HANDLE_ENV_MOBJ: usize = 1;
    pub const HANDLE_ARGS_MOBJ: usize = 2;
}

/// Parameters for the UpdateName message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UpdateNameQueryParameters {
    pub name: Buffer,
}

impl UpdateNameQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
}

/// Reply for the UpdateName message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UpdateNameReply {}

/// Parameters for the UpdateEnv message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UpdateEnvQueryParameters {}

impl UpdateEnvQueryParameters {
    pub const HANDLE_ENV_MOBJ: usize = 1;
}

/// Reply for the UpdateEnv message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UpdateEnvReply {}

/// Parameters for the SetExitCode message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetExitCodeQueryParameters {
    pub exit_code: i32,
}

/// Reply for the SetExitCode message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetExitCodeReply {}

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
