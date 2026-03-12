use crate::time::DateTime;
use core::{mem, ptr};

use alloc::fmt;

/// Name of the IPC port for the time server.
pub const PORT_NAME: &str = "time";

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
    ///
    /// Since IPC data structures are 8-bytes aligned, we use an unaligned byte array to store the i128 timestamp.
    pub timestamp: Timestamp,
}

/// A wrapper around the timestamp to allow unaligned access.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Timestamp([u8; mem::size_of::<i128>()]);

impl From<Timestamp> for i128 {
    fn from(value: Timestamp) -> Self {
        unsafe { ptr::read_unaligned(value.0.as_ptr() as *const i128) }
    }
}

impl TryFrom<Timestamp> for DateTime {
    type Error = ();

    fn try_from(value: Timestamp) -> Result<Self, Self::Error> {
        DateTime::from_unix_timestamp_nanos(i128::from(value)).map_err(|_| ())
    }
}

impl From<i128> for Timestamp {
    fn from(value: i128) -> Self {
        let mut timestamp = Self::default();
        unsafe {
            ptr::write_unaligned(timestamp.0.as_mut_ptr() as *mut i128, value);
        }
        timestamp
    }
}

impl From<DateTime> for Timestamp {
    fn from(value: DateTime) -> Self {
        Self::from(value.unix_timestamp_nanos())
    }
}
