mod engine;
mod handle;
mod init;
mod logging;
mod process;

use self::engine::register_syscall;

pub use engine::execute_syscall;
use syscalls::SyscallNumber;

pub fn init() {
    register_syscall(SyscallNumber::Log, logging::log);
    register_syscall(SyscallNumber::Close, handle::close);
    register_syscall(SyscallNumber::Duplicate, handle::duplicate);
    register_syscall(SyscallNumber::ProcessOpenSelf, process::open_self);
    register_syscall(SyscallNumber::ProcessCreate, process::create);
    register_syscall(SyscallNumber::ProcessMMap, process::mmap);
    register_syscall(SyscallNumber::ProcessMUnmap, process::munmap);
    register_syscall(SyscallNumber::ProcessMProtect, process::mprotect);

    register_syscall(SyscallNumber::InitSetup, init::setup);
}
