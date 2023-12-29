use log::trace;
use syscalls::{Error, SUCCESS};

use crate::interrupts::SyscallContext;

/// Wrapper around interrupts::SyscallContext to provide easier access
#[derive(Debug)]
pub struct Context {
    inner: SyscallContext,
}

/// Context for sync syscalls.
///
/// They do not set explicit result, it's their implementation function return value.
pub trait SyncContext {
    fn arg1(&self) -> usize;
    fn arg2(&self) -> usize;
    fn arg3(&self) -> usize;
    fn arg4(&self) -> usize;
    fn arg5(&self) -> usize;
    fn arg6(&self) -> usize;
}

impl Context {
    pub fn from(inner: SyscallContext) -> Self {
        Self { inner }
    }

    pub fn set_result(&self, result: Result<(), Error>) {
        trace!("Syscall ret={result:?}");

        let ret = match result {
            Ok(_) => SUCCESS,
            Err(err) => err as usize,
        };

        self.inner.set_result(ret);
    }
}

impl SyncContext for Context {
    fn arg1(&self) -> usize {
        self.inner.arg1()
    }

    fn arg2(&self) -> usize {
        self.inner.arg2()
    }

    fn arg3(&self) -> usize {
        self.inner.arg3()
    }

    fn arg4(&self) -> usize {
        self.inner.arg4()
    }

    fn arg5(&self) -> usize {
        self.inner.arg5()
    }

    fn arg6(&self) -> usize {
        self.inner.arg6()
    }
}
