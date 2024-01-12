pub const PAGE_SIZE: usize = 4096;

use core::fmt::Debug;
pub use libsyscalls::{
    Error, Exception, Handle, KallocStats, KvmStats, MemoryStats, Permissions, PhysStats,
    ProcessEvent, ProcessEventType, ProcessInfo, ThreadContext, ThreadContextRegister, ThreadEvent,
    ThreadEventType, ThreadInfo, ThreadPriority,
};

mod ipc;
mod listener;
mod memory;
mod memory_object;
mod process;
mod thread;
mod tls;

/// Trait to be implemented by all kobjects
pub trait KObject: Debug {
    /// Get the internal handle of the object
    unsafe fn handle(&self) -> &Handle;
}

pub use ipc::{KWaitable, Message, Port, PortReceiver, PortSender, Waiter};
pub use listener::{ProcessListener, ProcessListenerFilter, ThreadListener, ThreadListenerFilter};
pub use memory::Memory;
pub use memory_object::MemoryObject;
pub use process::{Mapping, Process};
pub use thread::{Thread, ThreadOptions, ThreadSupervisor};
pub use tls::{TlsAllocator, TlsSlot};
