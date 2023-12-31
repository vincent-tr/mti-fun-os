mod context;
mod engine;
mod handle;
mod helpers;
mod init;
mod ipc;
mod logging;
mod memory_object;
mod process;
mod thread;

use self::context::{Context, SyncContext};
use self::engine::{register_syscall, register_syscall_raw, register_syscall_sync};

pub use engine::execute_syscall;
use syscalls::SyscallNumber;

pub fn init() {
    register_syscall_sync(SyscallNumber::Log, logging::log);

    register_syscall_sync(SyscallNumber::HandleClose, handle::close);
    register_syscall_sync(SyscallNumber::HandleDuplicate, handle::duplicate);
    register_syscall_sync(SyscallNumber::HandleType, handle::r#type);

    register_syscall_sync(SyscallNumber::ProcessOpenSelf, &process::open_self);
    register_syscall_sync(SyscallNumber::ProcessOpen, process::open);
    register_syscall_sync(SyscallNumber::ProcessCreate, process::create);
    register_syscall_sync(SyscallNumber::ProcessMMap, process::mmap);
    register_syscall_sync(SyscallNumber::ProcessMUnmap, process::munmap);
    register_syscall_sync(SyscallNumber::ProcessMProtect, process::mprotect);
    register_syscall_sync(SyscallNumber::ProcessInfo, process::info);
    register_syscall_sync(SyscallNumber::ProcessList, process::list);

    register_syscall_sync(SyscallNumber::ThreadOpenSelf, thread::open_self);
    register_syscall_sync(SyscallNumber::ThreadOpen, thread::open);
    register_syscall_sync(SyscallNumber::ThreadCreate, thread::create);
    register_syscall(SyscallNumber::ThreadExit, thread::exit);
    register_syscall_sync(SyscallNumber::ThreadKill, thread::kill);
    register_syscall_sync(SyscallNumber::ThreadInfo, thread::info);
    register_syscall_sync(SyscallNumber::ThreadList, thread::list);

    register_syscall_sync(SyscallNumber::MemoryObjectCreate, memory_object::create);

    register_syscall_sync(SyscallNumber::PortOpen, ipc::open);
    register_syscall_sync(SyscallNumber::PortCreate, ipc::create);
    register_syscall_sync(SyscallNumber::PortInfo, ipc::info);
    register_syscall_sync(SyscallNumber::PortList, ipc::list);

    register_syscall_raw(SyscallNumber::InitSetup, init::setup);
}
