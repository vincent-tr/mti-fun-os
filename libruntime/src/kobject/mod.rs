pub const PAGE_SIZE: usize = 4096;

pub use libsyscalls::{Error, Handle, ThreadPriority};

mod ipc;
mod memory;
mod process;
mod thread;

/// Trait to be implemented by all kobjects
pub trait KObject {
    /// Get the internal handle of the object
    unsafe fn handle(&self) -> &Handle;
}

pub use ipc::{Message, Port, PortReceiver, PortSender, PortWaiter};
pub use memory::MemoryObject;
pub use process::Process;
pub use thread::{Thread, ThreadOptions};
