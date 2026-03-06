use syscalls::SyscallNumber;

use super::{Handle, IrqInfo, SyscallResult, syscalls::*, sysret_to_result};

/// Create a new IRQ handle for the given port
pub fn create(port: &Handle) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall2(
            SyscallNumber::IrqCreate,
            port.as_syscall_value(),
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn info(irq: &Handle) -> SyscallResult<IrqInfo> {
    let mut info_out = IrqInfo {
        msi_address: 0,
        vector: 0,
    };

    let ret = unsafe {
        syscall2(
            SyscallNumber::IrqInfo,
            irq.as_syscall_value(),
            &mut info_out as *mut _ as usize,
        )
    };

    sysret_to_result(ret)?;

    Ok(info_out)
}
