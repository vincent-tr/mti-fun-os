use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::debug;
use spin::RwLock;

use super::{error::not_supported, Error};

/// Type of a syscal; handler
pub type SyscallHandler = fn(usize, usize, usize, usize, usize, usize) -> Result<(), Error>;

const SUCCESS: usize = 0;

struct Handlers {
    handlers: HashMap<usize, SyscallHandler>,
}

impl Handlers {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, n: usize, handler: SyscallHandler) {
        assert!(self.handlers.insert(n, handler).is_none());
    }

    pub fn execute(
        &self,
        n: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
        arg4: usize,
        arg5: usize,
        arg6: usize,
    ) -> Result<(), Error> {
        if let Some(handler) = self.handlers.get(&n) {
            handler(arg1, arg2, arg3, arg4, arg5, arg6)
        } else {
            Err(not_supported())
        }
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
    debug!("Syscall {n} (arg1={arg1} (0x{arg1:016X}), arg2={arg2} (0x{arg2:016X}), arg3={arg3} (0x{arg3:016X}), arg4={arg4} (0x{arg4:016X}), arg5={arg5} (0x{arg5:016X}), arg6={arg6} (0x{arg6:016X}))");

    let handler = HANDLERS.read();

    let ret = match handler.execute(n, arg1, arg2, arg3, arg4, arg5, arg6) {
        Ok(_) => SUCCESS,
        Err(err) => err as usize,
    };

    debug!("Syscall ret={ret}");

    ret
}

/// Register a new syscall handler
pub fn register_syscall(n: usize, handler: SyscallHandler) {
    debug!("Add syscall {n}");
    let mut handlers = HANDLERS.write();
    handlers.register(n, handler);
}
