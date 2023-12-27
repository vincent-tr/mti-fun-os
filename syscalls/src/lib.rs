#![no_std]

mod error;
mod number;
mod permissions;

pub use error::{Error, SUCCESS};
pub use number::SyscallNumber;
pub use permissions::Permissions;
