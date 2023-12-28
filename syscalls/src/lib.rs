#![no_std]

mod error;
mod handle;
mod permissions;
mod process;
mod thread;

pub use error::{Error, SUCCESS};
pub use handle::HandleType;
pub use permissions::Permissions;
pub use process::ProcessInfo;
pub use thread::{ThreadInfo, ThreadPriority, ThreadState};

/// List of syscall numbers
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallNumber {
    Log = 1,

    HandleClose,
    HandleDuplicate,
    HandleType,

    ProcessOpenSelf,
    ProcessCreate,
    ProcessMMap,
    ProcessMUnmap,
    ProcessMProtect,
    ProcessList,

    ThreadOpenSelf,
    ThreadCreate,
    ThreadExit,
    ThreadKill,
    ThreadSetPriority,
    ThreadList,

    MemoryObjectCreate,

    InitSetup,
}
