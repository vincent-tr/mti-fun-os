mod engine;
mod handle;
mod logging;
mod process;

use self::engine::register_syscall;

pub use engine::execute_syscall;

/// List of syscall numbers
///
/// TODO: share with userland
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallNumber {
    Log = 1,
    Close,
    ProcessOpenSelf,
    ProcessCreate,
}

pub fn init() {
    register_syscall(SyscallNumber::Log, logging::log);
    register_syscall(SyscallNumber::Close, handle::close);
    register_syscall(SyscallNumber::ProcessOpenSelf, process::open_self);
    register_syscall(SyscallNumber::ProcessCreate, process::create);
}
