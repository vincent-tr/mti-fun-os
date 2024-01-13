use syscalls::{
    Exception, SyscallNumber, ThreadContext, ThreadContextRegister, ThreadCreationParameters,
    ThreadInfo, ThreadPriority,
};

use crate::SyscallInStr;

use super::{
    ref_ptr, syscalls::*, sysret_to_result, Handle, SyscallList, SyscallOutPtr, SyscallResult,
};

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
    name: Option<&str>,
    process: &Handle,
    privileged: bool,
    priority: ThreadPriority,
    entry_point: extern "C" fn(usize) -> !,
    stack_top: usize,
    arg: usize,
    tls: usize,
) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let name_reader = name.map(SyscallInStr::new);

    let params = ThreadCreationParameters {
        process_handle: unsafe { process.as_syscall_value() } as u64,
        privileged,
        priority,
        entry_point: entry_point as usize,
        stack_top,
        arg,
        tls,
    };

    let (ptr, len) = name_reader.as_ref().map_or((0, 0), |reader| unsafe {
        (reader.ptr_arg(), reader.len_arg())
    });

    let ret = unsafe {
        syscall4(
            SyscallNumber::ThreadCreate,
            ptr,
            len,
            ref_ptr(&params),
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
