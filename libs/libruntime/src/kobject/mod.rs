pub const PAGE_SIZE: usize = 4096;

use core::fmt::Debug;
pub use libsyscalls::{
    Exception, Handle, KallocStats, KvmStats, MemoryStats, Permissions, PhysStats, PortAccess,
    ProcessEvent, ProcessEventType, ProcessInfo, ThreadContext, ThreadContextRegister, ThreadEvent,
    ThreadEventType, ThreadInfo, ThreadPriority, TimerEvent,
};

mod error;
mod ioport;
mod ipc;
mod listener;
mod memory;
mod memory_object;
mod process;
mod thread;
mod timer;
mod tls;

/// Trait to be implemented by all kobjects
pub trait KObject: Debug {
    /// Get the internal handle of the object
    unsafe fn handle(&self) -> &Handle;

    /// Consume the object and return its handle
    fn into_handle(self) -> Handle;

    /// Safety: caller must ensure the handle is a valid handle of the implementing type
    unsafe fn from_handle_unchecked(handle: Handle) -> Self;

    /// Create the object from a handle
    fn from_handle(handle: Handle) -> Result<Self, Error>
    where
        Self: Sized;
}

pub use error::Error;
pub use ioport::PortRange;
pub use ipc::{KWaitable, Message, Port, PortReceiver, PortSender, Waiter};
pub use listener::{ProcessListener, ProcessListenerFilter, ThreadListener, ThreadListenerFilter};
pub use memory::Memory;
pub use memory_object::MemoryObject;
pub use process::{Mapping, Process};
pub use thread::{Thread, ThreadOptions, ThreadSupervisor};
pub use timer::Timer;
pub use tls::{TlsAllocator, TlsSlot};

pub mod helpers {
    // Used by init loader to setup process initial thread
    pub use super::thread::{AllocWithGuards, STACK_SIZE};
    pub use super::tls::TLS_SIZE;
}

pub(crate) fn init() {
    thread::ThreadRuntime::get().init();
}

pub(crate) fn terminate() {
    thread::ThreadRuntime::get().terminate();
}
