use syscalls::{
    Exception, SyscallNumber, ThreadContext, ThreadContextRegister, ThreadInfo, ThreadPriority,
};

use super::{syscalls::*, sysret_to_result, Handle, SyscallList, SyscallOutPtr, SyscallResult};

pub fn open_self() -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe { syscall1(SyscallNumber::ThreadOpenSelf, new_handle.as_syscall_ptr()) };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn open(tid: u64) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall2(
            SyscallNumber::ThreadOpen,
            tid as usize,
            new_handle.as_syscall_ptr(),
        )
    };

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

/// Get info about the thread
pub fn info(thread: &Handle) -> SyscallResult<ThreadInfo> {
    let info = SyscallOutPtr::new();

    let ret = unsafe {
        syscall2(
            SyscallNumber::ThreadInfo,
            thread.as_syscall_value(),
            info.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(info.take())
}

/// Get list of tids living in the system
pub fn list<'a>(array: &'a mut [u64]) -> SyscallResult<(&'a [u64], usize)> {
    let mut list = unsafe { SyscallList::new(array) };

    let ret = unsafe {
        syscall2(
            SyscallNumber::ThreadList,
            list.array_ptr_arg(),
            list.count_ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(list.finalize())
}

/// Get the error info of the thread
///
/// Note: the thread must be in error state
pub fn error_info(thread: &Handle) -> SyscallResult<Exception> {
    let error = SyscallOutPtr::new();

    let ret = unsafe {
        syscall2(
            SyscallNumber::ThreadErrorInfo,
            thread.as_syscall_value(),
            error.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(error.take())
}

/// Get the context of the thread
///
/// Note: the thread must be in error state
pub fn context(thread: &Handle) -> SyscallResult<ThreadContext> {
    let context = SyscallOutPtr::new();

    let ret = unsafe {
        syscall2(
            SyscallNumber::ThreadContext,
            thread.as_syscall_value(),
            context.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(context.take())
}

/// Update the context of the thread
///
/// Note: the thread must be in error state
pub fn update_context(
    thread: &Handle,
    regs: &[(ThreadContextRegister, usize)],
) -> SyscallResult<()> {
    let size = regs.len();

    let ret = unsafe {
        syscall3(
            SyscallNumber::ThreadUpdateContext,
            thread.as_syscall_value(),
            regs.as_ptr() as usize,
            size,
        )
    };

    sysret_to_result(ret)?;

    Ok(())
}

/// Resume the execution of the thread
///
/// Note: the thread must be in error state
pub fn resume(thread: &Handle) -> SyscallResult<()> {
    let ret = unsafe { syscall1(SyscallNumber::ThreadResume, thread.as_syscall_value()) };

    sysret_to_result(ret)
}
