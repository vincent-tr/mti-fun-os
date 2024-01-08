use syscalls::SyscallNumber;

use super::{slice_ptr, syscalls::*, sysret_to_result, Handle, SyscallResult};

pub fn create_process(port: &Handle, pids: Option<&[u64]>) -> SyscallResult<Handle> {
    let (pid_list_ptr, pid_list_size) = if let Some(list) = pids {
        assert!(list.len() > 0);

        (unsafe { slice_ptr(list) }, list.len())
    } else {
        (0, 0)
    };

    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall4(
            SyscallNumber::ListenerCreateProcess,
            port.as_syscall_value(),
            pid_list_ptr,
            pid_list_size,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn create_thread(port: &Handle, tids: Option<&[u64]>) -> SyscallResult<Handle> {
    let (tid_list_ptr, tid_list_size) = if let Some(list) = tids {
        assert!(list.len() > 0);

        (unsafe { slice_ptr(list) }, list.len())
    } else {
        (0, 0)
    };

    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall4(
            SyscallNumber::ListenerCreateThread,
            port.as_syscall_value(),
            tid_list_ptr,
            tid_list_size,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}
