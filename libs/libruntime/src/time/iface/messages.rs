use alloc::fmt;

/// Name of the IPC port for the time server.
pub const PORT_NAME: &str = "time-server";

/// Version of the time management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in time management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    GetWallTime = 1,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}

/// Errors used by the time server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum TimeServerError {
    InvalidArgument = 1,
    RuntimeError,
}

impl fmt::Display for TimeServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
        }
    }
}

/// Parameters for the GetWallTime message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetWallTimeQueryParameters {}

/// Reply for the GetWallTime message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetWallTimeReply {
    /// Timestamp in nanoseconds since the Unix epoch (January 1, 1970).
    pub timestamp: i128,
}
