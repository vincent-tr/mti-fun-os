use core::mem;

use super::{Context, SyncContext};
use alloc::sync::Arc;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::trace;
use spin::RwLock;
use syscalls::Error;

use crate::{interrupts::SyscallContext, user::error::not_supported};

use super::SyscallNumber;

/// Type of a syscal; handler
pub type SyscallHandler<'a> = &'a dyn FnMut(Context);

pub type SyscallSyncHandler<'a> = &'a dyn FnMut(&dyn SyncContext) -> Result<(), Error>;

struct Handlers {
    handlers: HashMap<SyscallNumber, Arc<SyscallHandler>>,
}

unsafe impl Send for Handlers {}
unsafe impl Sync for Handlers {}

impl Handlers {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, syscall_number: SyscallNumber, handler: SyscallHandler) {
        assert!(self
            .handlers
            .insert(syscall_number, Arc::from(handler))
            .is_none());
    }

    pub fn register_sync(&mut self, syscall_number: SyscallNumber, handler: SyscallSyncHandler) {
        self.register(syscall_number, move |context: Context| {
            let ret = handler(&context);
            context.set_result(ret);
        });
    }

    pub fn unregister(&mut self, syscall_number: SyscallNumber) {
        assert!(self.handlers.remove(&syscall_number).is_some());
    }

    pub fn get(&self, syscall_number: SyscallNumber) -> Option<Arc<SyscallHandler>> {
        self.handlers.get(&syscall_number).cloned()
    }
}

lazy_static! {
    static ref HANDLERS: RwLock<Handlers> = RwLock::new(Handlers::new());
}

/// Execute a system call
pub fn execute_syscall(n: usize, context: SyscallContext) {
    // If the number is not in struct we just won't get the key
    let syscall_number: SyscallNumber = unsafe { mem::transmute(n) };

    trace!("Syscall {syscall_number:?} {context:?}");

    let context = Context::from(context);

    // Do not keep the lock while executing, else we cannot register/unregister syscalls from a syscall
    let handler = {
        let handlers = HANDLERS.read();
        handlers.get(syscall_number)
    };

    if let Some(handler) = handler {
        handler(context);
    } else {
        context.set_result(Err(not_supported()));
    };
}

/// Register a new syscall handler
pub fn register_syscall(syscall_number: SyscallNumber, handler: SyscallHandler) {
    trace!("Add syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.register(syscall_number, handler);
}

/// Register a new synchronous syscall handler
pub fn register_syscall_sync(syscall_number: SyscallNumber, handler: SyscallSyncHandler) {
    trace!("Add syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.register_sync(syscall_number, handler);
}

/// Unregister a syscall handler
///
/// Used to remove the initial "init run" syscall
pub fn unregister_syscall(syscall_number: SyscallNumber) {
    trace!("Remove syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.unregister(syscall_number);
}
