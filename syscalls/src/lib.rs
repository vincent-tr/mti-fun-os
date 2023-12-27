#![no_std]

mod error;
mod number;
mod permissions;
mod thread_priority;

pub use error::{Error, SUCCESS};
pub use number::SyscallNumber;
pub use permissions::Permissions;
pub use thread_priority::ThreadPriority;
