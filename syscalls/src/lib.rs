#![no_std]

mod error;
mod handle;
mod ipc;
mod listener;
mod permissions;
mod process;
mod thread;

pub use error::{Error, SUCCESS};
pub use handle::HandleType;
pub use ipc::{Message, PortInfo};
pub use listener::{ProcessEvent, ProcessEventType, ThreadEvent, ThreadEventType};
pub use permissions::Permissions;
pub use process::ProcessInfo;
pub use thread::{
    Exception, ThreadContext, ThreadContextRegister, ThreadInfo, ThreadPriority, ThreadState,
};

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
    ProcessInfo,
    ProcessList,

    ThreadOpenSelf,
    ThreadOpen,
    ThreadCreate,
    ThreadExit,
    ThreadKill,
    ThreadSetPriority,
    ThreadInfo,
    ThreadList,
    ThreadErrorInfo,
    ThreadContext,
    ThreadUpdateContext,
    ThreadResume,

    MemoryObjectCreate,

    PortCreate,
    PortOpen,
    PortSend,
    PortReceive,
    PortWait,
    PortInfo,
    PortList,

    ListenerCreateProcess,
    ListenerCreateThread,

    InitSetup,
}
