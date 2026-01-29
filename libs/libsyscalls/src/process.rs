use core::ops::Range;

use syscalls::SyscallNumber;

use super::{
    syscalls::*, sysret_to_result, Handle, Permissions, ProcessInfo, SyscallInOutPtr, SyscallInStr,
    SyscallList, SyscallResult,
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

/// Address info in a process
#[derive(Debug)]
pub struct AddressInfo {
    pub perms: Permissions,
    pub mobj: Option<Handle>,
    pub offset: usize,
}

/// Get information about a virtual address in the process address space
pub fn minfo(process: &Handle, addr: usize) -> SyscallResult<AddressInfo> {
    let mut info_out = syscalls::AddressInfo {
        perms: Permissions::NONE,
        mobj: unsafe { Handle::invalid().as_syscall_value() } as u64,
        offset: 0,
    };

    let ret = unsafe {
        syscall3(
            SyscallNumber::ProcessMInfo,
            process.as_syscall_value(),
            addr as usize,
            &mut info_out as *mut _ as usize,
        )
    };

    sysret_to_result(ret)?;

    let mobj_handle = unsafe { Handle::from_raw(info_out.mobj) };
    let mobj_handle = if mobj_handle.valid() {
        Some(mobj_handle)
    } else {
        None
    };

    Ok(AddressInfo {
        perms: info_out.perms,
        mobj: mobj_handle,
        offset: info_out.offset,
    })
}

pub fn exit() -> SyscallResult<()> {
    let ret = unsafe { syscall0(SyscallNumber::ProcessExit) };

    sysret_to_result(ret)
}

pub fn kill(process: &Handle) -> SyscallResult<()> {
    let ret = unsafe { syscall1(SyscallNumber::ProcessKill, process.as_syscall_value()) };

    sysret_to_result(ret)
}

/// Get info about the process
pub fn info(process: &Handle) -> SyscallResult<ProcessInfo> {
    let info = SyscallInOutPtr::default();

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

/// Set the process name
pub fn set_name(process: &Handle, name: &str) -> SyscallResult<()> {
    let name_reader = SyscallInStr::new(name);
    let ret = unsafe {
        syscall3(
            SyscallNumber::ProcessSetName,
            process.as_syscall_value(),
            name_reader.ptr_arg(),
            name_reader.len_arg(),
        )
    };

    sysret_to_result(ret)
}

/// Get the process name
///
/// This can be useful is name is longer than 128 (truncated in info)
pub fn get_name<'a>(
    process: &Handle,
    name_buffer: &'a mut [u8],
) -> SyscallResult<(&'a [u8], usize)> {
    let mut list = unsafe { SyscallList::new(name_buffer) };

    let ret = unsafe {
        syscall3(
            SyscallNumber::ProcessGetName,
            process.as_syscall_value(),
            list.array_ptr_arg(),
            list.count_ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(list.finalize())
}
