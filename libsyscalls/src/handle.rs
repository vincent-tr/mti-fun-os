use syscalls::SyscallNumber;

use super::SyscallResult;

use super::{
    out_ptr,
    syscalls::{syscall1, syscall2},
    sysret_to_result,
};

/// Handle: Pointer to kernel object
#[derive(Debug)]
pub struct Handle(u64);

impl Handle {
    /// Construct a new invalid handle
    pub const fn invalid() -> Self {
        Handle(0)
    }

    /// Indicate is the handle is valid
    pub const fn valid(&self) -> bool {
        self.0 != 0
    }

    /// Reserved for syscalls implementations
    pub(crate) unsafe fn as_syscall_ptr(&mut self) -> usize {
        out_ptr(self)
    }

    /// Reserved for syscalls implementations
    pub(crate) unsafe fn as_syscall_value(&self) -> usize {
        self.0 as usize
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if self.valid() {
            close(self).expect("Could not close handle");
        }
    }
}

impl Clone for Handle {
    fn clone(&self) -> Self {
        duplicate(self).expect("Could not duplicate handle")
    }
}

fn close(handle: &Handle) -> SyscallResult<()> {
    let ret = unsafe { syscall1(SyscallNumber::Close, handle.0 as usize) };

    sysret_to_result(ret)
}

fn duplicate(handle: &Handle) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();

    let ret = unsafe {
        syscall2(
            SyscallNumber::ProcessOpenSelf,
            handle.0 as usize,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}
