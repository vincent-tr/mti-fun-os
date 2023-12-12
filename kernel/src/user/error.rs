use crate::memory::{is_page_aligned, is_userspace, VirtAddr};

#[derive(Debug)]
#[repr(usize)]
pub enum Error {
    InvalidArgument = 1,
    OutOfMemory,
    NotSupported,
}

pub fn check_arg(condition: bool) -> Result<(), Error> {
    if !condition {
        Err(Error::InvalidArgument)
    } else {
        Ok(())
    }
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

pub fn not_supported() -> Error {
    Error::NotSupported
}