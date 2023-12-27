#![no_std]

mod handle;
mod logging;
pub mod process;
mod syscalls;

use core::mem;

pub use handle::*;
pub use logging::*;

use ::syscalls::SUCCESS;
pub use ::syscalls::{Error, Permissions};

pub type SyscallResult<T> = Result<T, Error>;

/// # Safety
///
/// Borrowing rules unchecked. Do right before syscalls only.
unsafe fn out_ptr<T>(value: &mut T) -> usize {
    let ptr: *mut T = value;
    mem::transmute(ptr)
}

fn sysret_to_result(sysret: usize) -> SyscallResult<()> {
    match sysret {
        SUCCESS => Ok(()),
        err => Err(unsafe { mem::transmute(err) }),
    }
}
