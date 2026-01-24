/// List of errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Error {
    InvalidArgument = 1,
    OutOfMemory,
    NotSupported,
    MemoryAccessDenied,
    ObjectNotFound,
    ObjectNameDuplicate,
    ObjectClosed,
    ObjectNotReady,
}

pub const SUCCESS: usize = 0;
