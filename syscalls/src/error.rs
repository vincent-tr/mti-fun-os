/// List of errors
#[derive(Debug)]
#[repr(usize)]
pub enum Error {
    InvalidArgument = 1,
    OutOfMemory,
    NotSupported,
    MemoryAccessDenied,
}

pub const SUCCESS: usize = 0;
