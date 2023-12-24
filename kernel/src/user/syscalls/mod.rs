mod engine;
mod handle;
mod logging;
mod process;

use self::engine::register_syscall;

pub use engine::execute_syscall;
use syscalls::SyscallNumber;

pub fn init() {
    register_syscall(SyscallNumber::Log, logging::log);
    register_syscall(SyscallNumber::Close, handle::close);
    register_syscall(SyscallNumber::ProcessOpenSelf, process::open_self);
    register_syscall(SyscallNumber::ProcessCreate, process::create);
}