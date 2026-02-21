use syscalls::SyscallNumber;

use super::{syscalls::*, sysret_to_result, Handle, PortAccess, SyscallResult};

pub fn open(from: u16, count: usize, access: PortAccess) -> SyscallResult<Handle> {
    let mut handle = Handle::invalid();
    let ret = unsafe {
        syscall4(
            SyscallNumber::IoPortOpen,
            from as usize,
            count,
            access.bits() as usize,
            handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(handle)
}

pub fn read(port_range: &Handle, index: u16, word_size: u8) -> SyscallResult<usize> {
    let mut value_out = 0;
    let ret = unsafe {
        syscall4(
            SyscallNumber::IoPortRead,
            port_range.as_syscall_value(),
            index as usize,
            word_size as usize,
            (&mut value_out as *mut usize) as usize,
        )
    };

    sysret_to_result(ret)?;

    Ok(value_out)
}

pub fn write(port_range: &Handle, index: u16, word_size: u8, value: usize) -> SyscallResult<()> {
    let ret = unsafe {
        syscall4(
            SyscallNumber::IoPortWrite,
            port_range.as_syscall_value(),
            index as usize,
            word_size as usize,
            value,
        )
    };

    sysret_to_result(ret)?;

    Ok(())
}
