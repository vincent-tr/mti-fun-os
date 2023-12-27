#![no_std]

mod error;
mod number;

pub use error::{Error, SUCCESS};
pub use number::SyscallNumber;
