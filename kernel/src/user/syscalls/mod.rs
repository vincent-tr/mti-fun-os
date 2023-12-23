
mod engine;
mod logging;

use self::engine::register_syscall;

pub use engine::execute_syscall;

/// List of syscall numbers
/// 
/// TODO: share with userland
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallNumber {
    Log = 1,
}

pub fn init() {
    register_syscall(SyscallNumber::Log, logging::log);
}
