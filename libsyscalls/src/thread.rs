use syscalls::SyscallNumber;

use super::{syscalls::*, sysret_to_result, Handle, SyscallResult};

pub fn open_self() -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe { syscall1(SyscallNumber::ThreadOpenSelf, new_handle.as_syscall_ptr()) };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn create(process: &Handle, entry_point: usize, stack_top: usize) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall4(
            SyscallNumber::ThreadCreate,
            process.as_syscall_value(),
            entry_point,
            stack_top,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn exit() -> SyscallResult<()> {
    let ret = unsafe { syscall0(SyscallNumber::ThreadExit) };

    sysret_to_result(ret)
}

pub fn kill(thread: &Handle) -> SyscallResult<()> {
    let ret = unsafe { syscall1(SyscallNumber::ThreadKill, thread.as_syscall_value()) };

    sysret_to_result(ret)
}
