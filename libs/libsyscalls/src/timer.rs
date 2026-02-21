use syscalls::SyscallNumber;

use super::{Handle, SyscallInOutPtr, SyscallResult, syscalls::*, sysret_to_result};

pub fn create(port: &Handle, id: u64) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall3(
            SyscallNumber::TimerCreate,
            port.as_syscall_value(),
            id as usize,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn arm(timer: &Handle, deadline: u64) -> SyscallResult<()> {
    let ret = unsafe {
        syscall2(
            SyscallNumber::TimerArm,
            timer.as_syscall_value(),
            deadline as usize,
        )
    };

    sysret_to_result(ret)?;

    Ok(())
}

pub fn cancel(timer: &Handle) -> SyscallResult<()> {
    let ret = unsafe { syscall1(SyscallNumber::TimerCancel, timer.as_syscall_value()) };

    sysret_to_result(ret)?;

    Ok(())
}

pub fn now() -> SyscallResult<u64> {
    let size = SyscallInOutPtr::default();
    let ret = unsafe { syscall1(SyscallNumber::TimerNow, size.ptr_arg()) };

    sysret_to_result(ret)?;

    Ok(size.take())
}
