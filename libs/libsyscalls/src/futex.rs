use syscalls::SyscallNumber;

use super::{SyscallInOutPtr, SyscallResult, ref_ptr, syscalls::*, sysret_to_result};

/// Wait on a futex located at uaddr with expected value
pub fn wait(uaddr: &u32, expected: u32) -> SyscallResult<()> {
    let ret = unsafe { syscall2(SyscallNumber::FutexWait, ref_ptr(uaddr), expected as usize) };

    sysret_to_result(ret)
}

/// Wake up to count waiters on futex located at uaddr
///
/// Returns the number of woken up waiters
pub fn wake(uaddr: &u32, count: usize) -> SyscallResult<usize> {
    let count_ptr = SyscallInOutPtr::new(count);

    let ret = unsafe {
        syscall2(
            SyscallNumber::FutexWake,
            ref_ptr(uaddr),
            count_ptr.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(count_ptr.take())
}
