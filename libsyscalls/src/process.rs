use syscalls::SyscallNumber;

use super::{syscalls::syscall1, sysret_to_result, Handle, SyscallResult};

pub fn open_self() -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe { syscall1(SyscallNumber::ProcessOpenSelf, new_handle.as_syscall_ptr()) };

    sysret_to_result(ret)?;

    Ok(new_handle)
}
