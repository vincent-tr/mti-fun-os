use core::fmt;

use crate::{ipc::buffer_messages::Buffer, kobject};

/// Name of the IPC port for the state server.
pub const PORT_NAME: &str = "state-server";

/// Version of the state management messages.
pub const VERSION: u16 = 1;

/// Size of the state value.
pub const STATE_SIZE: usize = kobject::PAGE_SIZE;

/// Types of messages used in state management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    GetState = 1,
}

impl From<Type> for u16 {
    fn from(value: Type) -> Self {
        value as u16
    }
}
/// Errors used by the state server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum StateServerError {
    InvalidArgument = 1,
    RuntimeError,
}

impl fmt::Display for StateServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "InvalidArgument"),
            Self::RuntimeError => write!(f, "RuntimeError"),
        }
    }
}

/// Parameters for the GetState message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetStateQueryParameters {
    pub name: Buffer,
}

impl GetStateQueryParameters {
    pub const HANDLE_NAME_MOBJ: usize = 1;
}

/// Reply for the GetState message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetStateReply {}

impl GetStateReply {
    pub const HANDLE_VALUE_MOBJ: usize = 0;
}
