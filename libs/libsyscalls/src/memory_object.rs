use syscalls::SyscallNumber;

use super::{syscalls::*, sysret_to_result, Handle, SyscallInOutPtr, SyscallResult};

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

pub fn size(memory_object: &Handle) -> SyscallResult<usize> {
    let size = SyscallInOutPtr::default();
    let ret = unsafe {
        syscall2(
            SyscallNumber::MemoryObjectSize,
            memory_object.as_syscall_value(),
            size.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(size.take())
}
