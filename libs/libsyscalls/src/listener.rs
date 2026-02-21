use syscalls::SyscallNumber;

use super::{Handle, SyscallResult, slice_ptr, syscalls::*, sysret_to_result};

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

pub fn create_thread(port: &Handle, ids: Option<&[u64]>, is_pids: bool) -> SyscallResult<Handle> {
    let (id_list_ptr, id_list_size) = if let Some(list) = ids {
        assert!(list.len() > 0);

        (unsafe { slice_ptr(list) }, list.len())
    } else {
        (0, 0)
    };

    let is_pids = if is_pids { 1 } else { 0 };

    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall5(
            SyscallNumber::ListenerCreateThread,
            port.as_syscall_value(),
            id_list_ptr,
            id_list_size,
            is_pids,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}
