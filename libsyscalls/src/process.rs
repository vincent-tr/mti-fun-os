use core::ops::Range;

use syscalls::SyscallNumber;

use super::{
    syscalls::{syscall1, syscall3, syscall4, syscall6},
    sysret_to_result, Handle, Permissions, SyscallResult,
};

pub fn open_self() -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe { syscall1(SyscallNumber::ProcessOpenSelf, new_handle.as_syscall_ptr()) };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

pub fn create() -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();
    let ret = unsafe { syscall1(SyscallNumber::ProcessCreate, new_handle.as_syscall_ptr()) };

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
