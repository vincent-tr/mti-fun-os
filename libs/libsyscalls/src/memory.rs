use syscalls::SyscallNumber;

use super::{MemoryStats, SyscallInOutPtr, SyscallResult, syscalls::*, sysret_to_result};

/// Get info about the process
pub fn stats() -> SyscallResult<MemoryStats> {
    let stats = SyscallInOutPtr::default();

    let ret = unsafe { syscall1(SyscallNumber::MemoryStats, stats.ptr_arg()) };

    sysret_to_result(ret)?;

    Ok(stats.take())
}
