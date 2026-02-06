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
    GetProcessName,
    GetProcessEnv,
    GetProcessArgs,
    GetProcessStatus,
    TerminateProcess,
    RegisterProcessTerminatedNotification,
    UnregisterProcessTerminatedNotification,
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
    ProcessTerminated = 1,
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
    BufferTooSmall,
    ProcessNotRunning,
}

impl fmt::Display for ProcessServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::InvalidBinaryFormat => write!(f, "InvalidBinaryFormat"),
            Self::RuntimeError => write!(f, "RuntimeError"),
            Self::BufferTooSmall => write!(f, "BufferTooSmall"),
            Self::ProcessNotRunning => write!(f, "ProcessNotRunning"),
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

    /// PID of the created process
    pub pid: u64,
}

/// Parameters for the OpenProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenProcessQueryParameters {
    pub pid: u64,
}

/// Reply for the OpenProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OpenProcessReply {
    /// Server handle to the opened process
    pub handle: Handle,

    /// PID of the opened process
    pub pid: u64,
}

/// Parameters for the CloseProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseProcessQueryParameters {
    /// Handle to close
    pub handle: Handle,
}

/// Reply for the CloseProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CloseProcessReply {}

/// Parameters for the GetProcessName message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetProcessNameQueryParameters {
    /// Handle to the process
    pub handle: Handle,

    /// Buffer to write the name into (if the call succeeds)
    pub buffer: Buffer,
}

impl GetProcessNameQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 1; // Ownership kept by the client, server has write access
}

/// Reply for the GetProcessName message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetProcessNameReply {
    /// Length of the process name (if the call succeeds)
    pub buffer_used_len: usize,
}

/// Parameters for the GetProcessEnv message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetProcessEnvQueryParameters {
    /// Handle to the process
    pub handle: Handle,
}

/// Reply for the GetProcessEnv message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetProcessEnvReply {}

impl GetProcessEnvReply {
    pub const HANDLE_ENV_MOBJ: usize = 1; // Readonly, ownership kept by the server, KVBlock format
}

/// Parameters for the GetProcessArgs message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetProcessArgsQueryParameters {
    /// Handle to the process
    pub handle: Handle,
}

/// Reply for the GetProcessArgs message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetProcessArgsReply {}

impl GetProcessArgsReply {
    pub const HANDLE_ARGS_MOBJ: usize = 1; // Readonly, ownership kept by the server, KVBlock format
}

/// Parameters for the GetProcessStatus message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetProcessStatusQueryParameters {
    pub handle: Handle,
}

/// Reply for the GetProcessStatus message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetProcessStatusReply {
    pub status: ProcessStatus,
}

/// State of a process, as returned by the process server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    Running,
    Exited(i32), // exit code
}

/// Exit code used when a process exits successfully
pub const EXIT_CODE_SUCCESS: i32 = 0;

/// Exit code used when a process has not exited yet, or the exit code has not been reported by the process
pub const EXIT_CODE_UNSET: i32 = i32::MIN;

/// Exit code used when a process is killed
pub const EXIT_CODE_KILLED: i32 = i32::MIN + 1;

/// Parameters for the TerminateProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TerminateProcessQueryParameters {
    pub handle: Handle,
}

/// Reply for the TerminateProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TerminateProcessReply {}

/// Parameters for the ListProcesses message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListProcessesQueryParameters {
    /// Buffer to write the process list into (if the call succeeds)
    pub buffer: Buffer,
}

impl ListProcessesQueryParameters {
    pub const HANDLE_BUFFER_MOBJ: usize = 1; // Readonly, ownership kept by the client, server has write access, ProcessListBlock format
}

/// Reply for the ListProcesses message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ListProcessesReply {
    /// Number of bytes used in the buffer to write the process list (if the call succeeds)
    pub buffer_used_len: usize,
}

/// Parameters for the RegisterProcessTerminatedNotification message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RegisterProcessTerminatedNotificationQueryParameters {
    /// Handle to the process to monitor
    pub handle: Handle,

    /// Value to correlate the notification with the registration
    /// 
    /// This value will be sent back in the ProcessTerminatedNotification
    pub correlation: u64,
}

impl RegisterProcessTerminatedNotificationQueryParameters {
    pub const HANDLE_PORT: usize = 1;
}

/// Reply for the RegisterProcessTerminatedNotification message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RegisterProcessTerminatedNotificationReply {
    /// Handle to the notification registration, used for unregistering later
    pub registration_handle: Handle,
}

/// Parameters for the UnregisterProcessTerminatedNotification message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UnregisterProcessTerminatedNotificationQueryParameters {
    /// Handle to the notification registration to cancel
    pub registration_handle: Handle,
}

/// Reply for the UnregisterProcessTerminatedNotification message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UnregisterProcessTerminatedNotificationReply {}

/// Notification parameters for the ProcessTerminated notification.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ProcessTerminatedNotification {
    /// Value to correlate the notification with the registration
    /// 
    /// This value was provided in the RegisterProcessTerminatedNotification message
    pub correlation: u64,

    /// PID of the terminated process
    pub pid: u64,

    /// Exit code of the terminated process (if available, otherwise EXIT_CODE_UNSET)
    pub exit_code: i32,
}
