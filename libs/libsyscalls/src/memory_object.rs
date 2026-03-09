use syscalls::SyscallNumber;

use super::{Handle, IoMemFlags, SyscallInOutPtr, SyscallResult, syscalls::*, sysret_to_result};

pub fn create(size: usize) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall2(
            SyscallNumber::MemoryObjectCreate,
            size,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn open_iomem(
    phys_addr: usize,
    size: usize,
    write_through: bool,
    no_cache: bool,
) -> SyscallResult<Handle> {
    let mut flags = IoMemFlags::NONE;
    if write_through {
        flags |= IoMemFlags::WRITE_THROUGH;
    }
    if no_cache {
        flags |= IoMemFlags::NO_CACHE;
    }

    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall4(
            SyscallNumber::MemoryObjectOpenIoMem,
            phys_addr,
            size,
            flags.bits() as usize,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn size(memory_object: &Handle) -> SyscallResult<usize> {
    let size = SyscallInOutPtr::default();
    let ret = unsafe {
        syscall2(
            SyscallNumber::MemoryObjectSize,
            memory_object.as_syscall_value(),
            size.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(size.take())
}

pub fn phys_addr(memory_object: &Handle, offset: usize) -> SyscallResult<usize> {
    let phys_addr = SyscallInOutPtr::default();
    let ret = unsafe {
        syscall3(
            SyscallNumber::MemoryObjectPhysAddr,
            memory_object.as_syscall_value(),
            offset,
            phys_addr.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(phys_addr.take())
}
