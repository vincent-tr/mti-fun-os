use syscalls::SyscallNumber;

use super::{syscalls::*, sysret_to_result, MemoryStats, SyscallOutPtr, SyscallResult};

/// Get info about the process
pub fn stats() -> SyscallResult<MemoryStats> {
    let stats = SyscallOutPtr::new();

    let ret = unsafe { syscall1(SyscallNumber::MemoryStats, stats.ptr_arg()) };

    sysret_to_result(ret)?;

    Ok(stats.take())
}
