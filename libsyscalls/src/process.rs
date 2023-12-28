use core::ops::Range;

use syscalls::{ProcessInfo, SyscallNumber};

use super::{
    syscalls::*, sysret_to_result, Handle, Permissions, SyscallInStr, SyscallList, SyscallOutPtr,
    SyscallResult,
};

pub fn open_self() -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe { syscall1(SyscallNumber::ProcessOpenSelf, new_handle.as_syscall_ptr()) };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn open(pid: u64) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe {
        syscall2(
            SyscallNumber::ProcessOpen,
            pid as usize,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn create(name: &str) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let name_reader = SyscallInStr::new(name);
    let ret = unsafe {
        syscall3(
            SyscallNumber::ProcessCreate,
            name_reader.ptr_arg(),
            name_reader.len_arg(),
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

/// Map a MemoryObject (or part of it) into the process address space, with the given permissions.
///
/// Notes:
/// - If `addr` is not set, an address where the mapping can fit will be found.
/// - If `addr` is set, this function cannot overwrite part of an existing mapping. Call unmap() before.
pub fn mmap(
    process: &Handle,
    addr: Option<usize>,
    size: usize,
    perms: Permissions,
    memory_object: Option<&Handle>,
    offset: usize,
) -> SyscallResult<usize> {
    let mut addr = if let Some(value) = addr { value } else { 0 };

    let (memory_object, offset) = if let Some(handle) = memory_object {
        (unsafe { handle.as_syscall_value() }, offset)
    } else {
        (unsafe { Handle::invalid().as_syscall_value() }, 0)
    };

    // Note: addr is modified in-place
    let addr_ptr = &mut addr as *mut _;
    let ret = unsafe {
        syscall6(
            SyscallNumber::ProcessMMap,
            process.as_syscall_value(),
            addr_ptr as usize,
            size,
            perms.bits() as usize,
            memory_object,
            offset,
        )
    };

    sysret_to_result(ret)?;

    Ok(addr)
}

/// Unmap the address space from addr to addr+size.
///
/// Notes:
/// - It may contains multiple mappings,
/// - addr or addr+size may be in the middle of a mapping
/// - part of the specified area my not be mapped. In consequence, calling unmap() on an unmapped area is a successful noop.
///
pub fn munmap(process: &Handle, range: &Range<usize>) -> SyscallResult<()> {
    let ret = unsafe {
        syscall3(
            SyscallNumber::ProcessMUnmap,
            process.as_syscall_value(),
            range.start as usize,
            range.len(),
        )
    };

    sysret_to_result(ret)
}

/// Change the permissions for the given memory region
///
/// Notes:
/// - It can only contains one mapping
/// - The mapping may be larger than the given region. It will be split.
pub fn mprotect(process: &Handle, range: &Range<usize>, perms: Permissions) -> SyscallResult<()> {
    let ret = unsafe {
        syscall4(
            SyscallNumber::ProcessMProtect,
            process.as_syscall_value(),
            range.start as usize,
            range.len(),
            perms.bits() as usize,
        )
    };

    sysret_to_result(ret)
}

/// Get info about the process
pub fn info(process: &Handle) -> SyscallResult<ProcessInfo> {
    let info = SyscallOutPtr::new();

    let ret = unsafe {
        syscall2(
            SyscallNumber::ProcessInfo,
            process.as_syscall_value(),
            info.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(info.take())
}

/// Get list of pids living in the system
pub fn list<'a>(array: &'a mut [u64]) -> SyscallResult<(&'a [u64], usize)> {
    let mut list = unsafe { SyscallList::new(array) };

    let ret = unsafe {
        syscall2(
            SyscallNumber::ProcessList,
            list.array_ptr_arg(),
            list.count_ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(list.finalize())
}
