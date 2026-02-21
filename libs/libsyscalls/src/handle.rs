use syscalls::{HandleType, SyscallNumber};

use super::SyscallResult;

use super::{ref_ptr, syscalls::*, sysret_to_result};

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
    pub unsafe fn as_syscall_ptr(&mut self) -> usize {
        unsafe { ref_ptr(self) }
    }

    /// Reserved for syscalls implementations
    pub const unsafe fn as_syscall_value(&self) -> usize {
        self.0 as usize
    }

    /// Build a pointer from a raw value. Reserved for ipc message implementations
    pub unsafe fn from_raw(value: u64) -> Self {
        Self(value)
    }

    pub fn r#type(&self) -> HandleType {
        if !self.valid() {
            HandleType::Invalid
        } else {
            r#type(self).expect("Could not get handle type")
        }
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
    let ret = unsafe { syscall1(SyscallNumber::HandleClose, handle.as_syscall_value()) };

    sysret_to_result(ret)
}

fn duplicate(handle: &Handle) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();

    let ret = unsafe {
        syscall2(
            SyscallNumber::HandleDuplicate,
            handle.as_syscall_value(),
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

fn r#type(handle: &Handle) -> SyscallResult<HandleType> {
    let mut handle_type = HandleType::Invalid;

    let ret = unsafe {
        syscall2(
            SyscallNumber::HandleType,
            handle.as_syscall_value(),
            ref_ptr(&mut handle_type),
        )
    };

    sysret_to_result(ret)?;

    Ok(handle_type)
}
