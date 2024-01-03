use core::{future::Future, mem, task};

use super::Context;
use alloc::{boxed::Box, sync::Arc};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::trace;
use spin::RwLock;
use syscalls::{Error, SUCCESS};

use crate::{
    interrupts::SyscallContext,
    user::{error::not_supported, thread},
};

use super::SyscallNumber;

/// Type of a raw syscall handler (init handler)
pub trait SyscallRawHandler = Fn(SyscallContext) + 'static;

/// Type of a syscall handler
pub trait SyscallHandler: (Fn(Context) -> Self::Future) + 'static {
    type Future: Future<Output = Result<(), Error>> + 'static;
}

impl<F, Fut> SyscallHandler for F
where
    F: (Fn(Context) -> Fut) + 'static,
    Fut: Future<Output = Result<(), Error>> + 'static,
{
    type Future = Fut;
}

struct Handlers {
    handlers: HashMap<SyscallNumber, Arc<dyn SyscallRawHandler>>,
}

unsafe impl Send for Handlers {}
unsafe impl Sync for Handlers {}

impl Handlers {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register<Handler: SyscallRawHandler>(
        &mut self,
        syscall_number: SyscallNumber,
        handler: Handler,
    ) {
        assert!(self
            .handlers
            .insert(syscall_number, Arc::from(handler))
            .is_none());
    }

    pub fn unregister(&mut self, syscall_number: SyscallNumber) {
        assert!(self.handlers.remove(&syscall_number).is_some());
    }

    pub fn get(&self, syscall_number: SyscallNumber) -> Option<Arc<dyn SyscallRawHandler>> {
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

    // Do not keep the lock while executing, else we cannot register/unregister syscalls from a syscall
    let handler = {
        let handlers = HANDLERS.read();
        handlers.get(syscall_number)
    };

    if let Some(handler) = handler {
        handler(context);
    } else {
        SyscallContext::set_current_result(not_supported() as usize);
    };
}

/// Register a new syscall handler
pub fn register_syscall_raw<Handler: SyscallRawHandler>(
    syscall_number: SyscallNumber,
    handler: Handler,
) {
    trace!("Add syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.register(syscall_number, handler);
}

/// Register a new syscall handler
pub fn register_syscall<Handler: SyscallHandler>(syscall_number: SyscallNumber, handler: Handler) {
    register_syscall_raw(syscall_number, move |inner: SyscallContext| {
        let context = Context::from(inner, &thread::current_thread());
        let mut future = Box::pin(handler(context));

        let waker = task::Waker::noop();
        let mut ctx = task::Context::from_waker(&waker);

        match future.as_mut().poll(&mut ctx) {
            task::Poll::Ready(result) => {
                // Syscall completed synchronously
                trace!("Syscall ret={result:?}");
                SyscallContext::set_current_result(prepare_result(result));
            }
            task::Poll::Pending => {
                todo!();
            }
        }
    });
}

pub fn prepare_result(result: Result<(), Error>) -> usize {
    match result {
        Ok(_) => SUCCESS,
        Err(err) => err as usize,
    }
}

/// Unregister a syscall handler
///
/// Used to remove the initial "init run" syscall
pub fn unregister_syscall(syscall_number: SyscallNumber) {
    trace!("Remove syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.unregister(syscall_number);
}
