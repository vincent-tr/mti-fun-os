use crate::memory::{is_page_aligned, is_userspace, VirtAddr, Permissions};

#[derive(Debug)]
#[repr(usize)]
pub enum Error {
    InvalidArgument = 1,
    OutOfMemory,
    NotSupported,
    MemoryAccessDenied,
}

pub fn invalid_argument() -> Error {
    Error::InvalidArgument
}

pub fn check_arg(condition: bool) -> Result<(), Error> {
    if !condition {
        Err(invalid_argument())
    } else {
        Ok(())
    }
}

pub fn check_arg_res<T, E>(res: Result<T, E>) -> Result<T, Error> {
    res.map_err(|_| invalid_argument())
}

pub fn check_is_userspace(addr: VirtAddr) -> Result<(), Error> {
    check_arg(is_userspace(addr))
}

pub fn check_page_alignment(addr: usize) -> Result<(), Error> {
    check_arg(is_page_aligned(addr))
}

pub fn check_positive(value: usize) -> Result<(), Error> {
    check_arg(value > 0)
}

pub fn out_of_memory() -> Error {
    Error::OutOfMemory
}

/// Check that actual permissions match at least expected
pub fn check_permissions(actual: Permissions, expected: Permissions) -> Result<(), Error> {
    for perm in [Permissions::READ, Permissions::WRITE, Permissions::EXECUTE] {
        if expected.contains(perm) && !actual.contains(perm) {
            return Err(Error::MemoryAccessDenied);
        }
    }

    Ok(())
}

pub fn not_supported() -> Error {
    Error::NotSupported
}
