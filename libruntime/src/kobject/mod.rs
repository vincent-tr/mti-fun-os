pub const PAGE_SIZE: usize = 4096;

use core::fmt::Debug;
pub use libsyscalls::{Error, Handle, ThreadPriority};

mod ipc;
//mod listener;
mod memory;
mod process;
mod thread;

/// Trait to be implemented by all kobjects
pub trait KObject: Debug {
    /// Get the internal handle of the object
    unsafe fn handle(&self) -> &Handle;
}

pub use ipc::{KWaitable, Message, Port, PortReceiver, PortSender, Waiter};
pub use memory::MemoryObject;
pub use process::Process;
pub use thread::{Thread, ThreadOptions};
