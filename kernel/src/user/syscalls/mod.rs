mod context;
mod engine;
mod handle;
mod helpers;
mod init;
mod ipc;
mod listener;
mod logging;
mod memory_object;
mod process;
mod thread;

pub use self::context::Context;
use self::engine::{register_syscall, register_syscall_raw};

pub use engine::{execute_syscall, SyscallExecutor};
use engine::{exit, sleep};
use syscalls::SyscallNumber;

pub fn init() {
    register_syscall(SyscallNumber::Log, logging::log);

    register_syscall(SyscallNumber::HandleClose, handle::close);
    register_syscall(SyscallNumber::HandleDuplicate, handle::duplicate);
    register_syscall(SyscallNumber::HandleType, handle::r#type);

    register_syscall(SyscallNumber::ProcessOpenSelf, process::open_self);
    register_syscall(SyscallNumber::ProcessOpen, process::open);
    register_syscall(SyscallNumber::ProcessCreate, process::create);
    register_syscall(SyscallNumber::ProcessMMap, process::mmap);
    register_syscall(SyscallNumber::ProcessMUnmap, process::munmap);
    register_syscall(SyscallNumber::ProcessMProtect, process::mprotect);
    register_syscall(SyscallNumber::ProcessInfo, process::info);
    register_syscall(SyscallNumber::ProcessList, process::list);

    register_syscall(SyscallNumber::ThreadOpenSelf, thread::open_self);
    register_syscall(SyscallNumber::ThreadOpen, thread::open);
    register_syscall(SyscallNumber::ThreadCreate, thread::create);
    register_syscall(SyscallNumber::ThreadExit, thread::exit);
    register_syscall(SyscallNumber::ThreadKill, thread::kill);
    register_syscall(SyscallNumber::ThreadInfo, thread::info);
    register_syscall(SyscallNumber::ThreadList, thread::list);

    register_syscall(SyscallNumber::MemoryObjectCreate, memory_object::create);

    register_syscall(SyscallNumber::PortOpen, ipc::open);
    register_syscall(SyscallNumber::PortCreate, ipc::create);
    register_syscall(SyscallNumber::PortSend, ipc::send);
    register_syscall(SyscallNumber::PortReceive, ipc::receive);
    register_syscall(SyscallNumber::PortWait, ipc::wait);
    register_syscall(SyscallNumber::PortInfo, ipc::info);
    register_syscall(SyscallNumber::PortList, ipc::list);

    register_syscall(
        SyscallNumber::ListenerCreateProcess,
        listener::create_process,
    );
    register_syscall(SyscallNumber::ListenerCreateThread, listener::create_thread);

    register_syscall_raw(SyscallNumber::InitSetup, init::setup);
}
