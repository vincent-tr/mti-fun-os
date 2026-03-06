#![no_std]

mod error;
mod handle;
pub mod init;
mod ioport;
mod ipc;
mod irq;
mod listener;
mod memory;
mod memory_object;
mod permissions;
mod process;
mod thread;
mod timer;

pub use error::*;
pub use handle::*;
pub use ioport::*;
pub use ipc::*;
pub use irq::*;
pub use listener::*;
pub use memory::*;
pub use memory_object::*;
pub use permissions::*;
pub use process::*;
pub use thread::*;
pub use timer::*;

/// List of syscall numbers
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallNumber {
    Log = 1,

    HandleClose,
    HandleDuplicate,
    HandleType,

    ProcessOpenSelf,
    ProcessOpen,
    ProcessCreate,
    ProcessMMap,
    ProcessMUnmap,
    ProcessMProtect,
    ProcessMInfo,
    ProcessExit,
    ProcessKill,
    ProcessInfo,
    ProcessList,
    ProcessSetName,
    ProcessGetName,

    ThreadOpenSelf,
    ThreadOpen,
    ThreadCreate,
    ThreadExit,
    ThreadKill,
    ThreadSetPriority,
    ThreadInfo,
    ThreadList,
    ThreadSetName,
    ThreadGetName,
    ThreadErrorInfo,
    ThreadContext,
    ThreadUpdateContext,
    ThreadResume,

    MemoryObjectCreate,
    MemoryObjectOpenIoMem,
    MemoryObjectSize,

    PortCreate,
    PortOpen,
    PortSend,
    PortReceive,
    PortWait,
    PortInfo,
    PortList,

    ListenerCreateProcess,
    ListenerCreateThread,

    TimerCreate,
    TimerArm,
    TimerCancel,
    TimerNow,

    InitSetup,

    MemoryStats,

    FutexWait,
    FutexWake,

    IoPortOpen,
    IoPortWrite,
    IoPortRead,
}
