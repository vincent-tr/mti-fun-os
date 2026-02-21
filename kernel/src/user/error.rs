use crate::memory::{Permissions, VirtAddr, is_page_aligned, is_userspace};

pub use syscalls::Error;

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

pub fn check_arg_opt<T>(value: Option<T>) -> Result<T, Error> {
    value.ok_or(invalid_argument())
}

pub fn check_is_userspace(addr: VirtAddr) -> Result<VirtAddr, Error> {
    check_arg(is_userspace(addr))?;
    Ok(addr)
}

pub fn check_page_alignment(addr: usize) -> Result<usize, Error> {
    check_arg(is_page_aligned(addr))?;
    Ok(addr)
}

pub fn check_positive(value: usize) -> Result<usize, Error> {
    check_arg(value > 0)?;
    Ok(value)
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

/// Check that actual permissions contains any permission (ie: not NONE)
pub fn check_any_permissions(perms: Permissions) -> Result<(), Error> {
    if perms == Permissions::NONE {
        Err(Error::MemoryAccessDenied)
    } else {
        Ok(())
    }
}

pub fn not_supported() -> Error {
    Error::NotSupported
}

pub fn check_found<T>(value: Option<T>) -> Result<T, Error> {
    value.ok_or(Error::ObjectNotFound)
}

pub fn duplicate_name() -> Error {
    Error::ObjectNameDuplicate
}

pub fn object_closed() -> Error {
    Error::ObjectClosed
}

pub fn object_not_ready() -> Error {
    Error::ObjectNotReady
}
