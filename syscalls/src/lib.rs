#![no_std]

mod error;
mod handle_type;
mod permissions;
mod thread_priority;

pub use error::{Error, SUCCESS};
pub use handle_type::HandleType;
pub use permissions::Permissions;
pub use thread_priority::ThreadPriority;

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
