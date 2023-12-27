#![no_std]

mod error;
mod permissions;
mod thread_priority;

pub use error::{Error, SUCCESS};
pub use permissions::Permissions;
pub use thread_priority::ThreadPriority;

/// List of syscall numbers
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallNumber {
    Log = 1,
    Close,
    Duplicate,
    ProcessOpenSelf,
    ProcessCreate,
    ProcessMMap,
    ProcessMUnmap,
    ProcessMProtect,
    ThreadOpenSelf,
    ThreadCreate,
    ThreadExit,
    ThreadKill,
    ThreadSetPriority,

    InitSetup,
}
