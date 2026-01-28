use alloc::fmt;

/// Name of the IPC port for the process server.
pub const PORT_NAME: &str = "process-server";

/// Version of the process management messages.
pub const VERSION: u16 = 1;

/// Types of messages used in process management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Type {
    CreateProcess = 1,
    GetStartupInfo = 3,
    UpdateProcessName = 2,
    UpdateEnv = 4,
    SetExitCode = 5,
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
}

impl fmt::Display for ProcessServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessServerError::InvalidArgument => write!(f, "InvalidArgument"),
        }
    }
}

/// Parameters for the CreateProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateProcessQueryParameters {
    pub name: Buffer,
    // Handle[1]: name mem_obj
    // Handle[2]: binary mem_obj - process-server will take ownership - data must start at offset 0
    // Handle[3]: env mem_obj - process-server will take ownership - data must start at offset 0
    // Handle[4]: args mem_obj - process-server will take ownership - data must start at offset 0
}

/// Reply for the CreateProcess message.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateProcessReply {
    pub pid: u32,
    // Handle[0]: create process handle, must close when done
    pub tid: u32,
    // Handle[1]: main thread handle, must close when done
}

/// A buffer descriptor used in IPC messages.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Buffer {
    // will take a handle slot for MemoryObject
    pub offset: usize,
    pub size: usize,
}
