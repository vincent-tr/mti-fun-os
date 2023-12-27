use core::mem;

use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::{debug, trace};
use spin::RwLock;
use syscalls::SUCCESS;

use crate::user::error::{not_supported, Error};

use super::SyscallNumber;

/// Type of a syscal; handler
pub type SyscallHandler = fn(usize, usize, usize, usize, usize, usize) -> Result<(), Error>;

struct Handlers {
    handlers: HashMap<SyscallNumber, SyscallHandler>,
}

impl Handlers {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, syscall_number: SyscallNumber, handler: SyscallHandler) {
        assert!(self.handlers.insert(syscall_number, handler).is_none());
    }

    pub fn unregister(&mut self, syscall_number: SyscallNumber) {
        assert!(self.handlers.remove(&syscall_number).is_some());
    }

    pub fn get(&self, syscall_number: SyscallNumber) -> Option<SyscallHandler> {
        self.handlers.get(&syscall_number).copied()
    }
}

lazy_static! {
    static ref HANDLERS: RwLock<Handlers> = RwLock::new(Handlers::new());
}

/// Execute a system call
pub fn execute_syscall(
    n: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    // If the number is not in struct we just won't get the key
    let syscall_number: SyscallNumber = unsafe { mem::transmute(n) };

    trace!("Syscall {syscall_number:?} (arg1={arg1} (0x{arg1:016X}), arg2={arg2} (0x{arg2:016X}), arg3={arg3} (0x{arg3:016X}), arg4={arg4} (0x{arg4:016X}), arg5={arg5} (0x{arg5:016X}), arg6={arg6} (0x{arg6:016X}))");

    // Do not keep the lock while executing, else we cannot register/unregister syscalls from a syscall
    let handler = {
        let handlers = HANDLERS.read();
        handlers.get(syscall_number)
    };

    let result = if let Some(handler) = handler {
        handler(arg1, arg2, arg3, arg4, arg5, arg6)
    } else {
        Err(not_supported())
    };

    let ret = match result {
        Ok(_) => SUCCESS,
        Err(err) => err as usize,
    };

    trace!("Syscall ret={ret}");

    ret
}

/// Register a new syscall handler
pub fn register_syscall(syscall_number: SyscallNumber, handler: SyscallHandler) {
    debug!("Add syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.register(syscall_number, handler);
}

/// Unregister a syscall handler
///
/// Used to remove the initial "init run" syscall
pub fn unregister_syscall(syscall_number: SyscallNumber) {
    debug!("Remove syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.unregister(syscall_number);
}
