use super::{SyscallResult, syscalls::*, sysret_to_result};
use syscalls::SyscallNumber;

pub fn log(level: log::Level, message: &str) -> SyscallResult<()> {
    let ret = unsafe {
        syscall3(
            SyscallNumber::Log,
            level as usize,
            message.as_ptr() as usize,
            message.len(),
        )
    };

    sysret_to_result(ret)
}
