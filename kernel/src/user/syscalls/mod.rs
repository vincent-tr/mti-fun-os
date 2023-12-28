mod engine;
mod handle;
mod helpers;
mod init;
mod logging;
mod memory_object;
mod process;
mod thread;

use self::engine::register_syscall;

pub use engine::execute_syscall;
use syscalls::SyscallNumber;

pub fn init() {
    register_syscall(SyscallNumber::Log, logging::log);

    register_syscall(SyscallNumber::HandleClose, handle::close);
    register_syscall(SyscallNumber::HandleDuplicate, handle::duplicate);
    register_syscall(SyscallNumber::HandleType, handle::r#type);

    register_syscall(SyscallNumber::ProcessOpenSelf, process::open_self);
    register_syscall(SyscallNumber::ProcessCreate, process::create);
    register_syscall(SyscallNumber::ProcessMMap, process::mmap);
    register_syscall(SyscallNumber::ProcessMUnmap, process::munmap);
    register_syscall(SyscallNumber::ProcessMProtect, process::mprotect);
    register_syscall(SyscallNumber::ProcessList, process::list);

    register_syscall(SyscallNumber::ThreadOpenSelf, thread::open_self);
    register_syscall(SyscallNumber::ThreadCreate, thread::create);
    register_syscall(SyscallNumber::ThreadExit, thread::exit);
    register_syscall(SyscallNumber::ThreadKill, thread::kill);
    register_syscall(SyscallNumber::ThreadList, thread::list);

    register_syscall(SyscallNumber::MemoryObjectCreate, memory_object::create);

    register_syscall(SyscallNumber::InitSetup, init::setup);
}
