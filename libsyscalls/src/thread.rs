use syscalls::{SyscallNumber, ThreadPriority};

use super::{syscalls::*, sysret_to_result, Handle, SyscallResult};

pub fn open_self() -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe { syscall1(SyscallNumber::ThreadOpenSelf, new_handle.as_syscall_ptr()) };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn create(
    process: &Handle,
    priority: ThreadPriority,
    entry_point: fn() -> !,
    stack_top: usize,
) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall5(
            SyscallNumber::ThreadCreate,
            process.as_syscall_value(),
            priority as usize,
            entry_point as usize,
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

pub fn set_priority(thread: &Handle, priority: ThreadPriority) -> SyscallResult<()> {
    let ret = unsafe {
        syscall2(
            SyscallNumber::ThreadSetPriority,
            thread.as_syscall_value(),
            priority as usize,
        )
    };

    sysret_to_result(ret)
}
