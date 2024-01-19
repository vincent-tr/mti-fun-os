use syscalls::SyscallNumber;

use super::{syscalls::*, sysret_to_result, Handle, SyscallResult};

pub fn create(size: usize) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall2(
            SyscallNumber::MemoryObjectCreate,
            size,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}
