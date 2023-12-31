use alloc::sync::{Arc, Weak};
use log::trace;
use syscalls::{Error, SUCCESS};

use crate::{interrupts::SyscallContext, user::thread::Thread};

/// Wrapper around interrupts::SyscallContext to provide easier access
#[derive(Debug)]
pub struct Context {
    inner: SyscallContext,
    owner: Weak<Thread>,
}

impl Context {
    pub fn from(inner: SyscallContext, thread: &Arc<Thread>) -> Self {
        Self {
            inner,
            owner: Arc::downgrade(thread),
        }
    }

    /// Only used from engine
    pub fn set_sync_result(&self, result: Result<(), Error>) {
        assert!(self.owner().state().is_executing());

        trace!("Syscall ret={result:?}");

        let ret = match result {
            Ok(_) => SUCCESS,
            Err(err) => err as usize,
        };

        SyscallContext::set_current_result(ret);
    }

    pub fn owner(&self) -> Arc<Thread> {
        self.owner.upgrade().expect("Context lost his owner thread")
    }

    pub fn arg1(&self) -> usize {
        self.inner.arg1()
    }

    pub fn arg2(&self) -> usize {
        self.inner.arg2()
    }

    pub fn arg3(&self) -> usize {
        self.inner.arg3()
    }

    pub fn arg4(&self) -> usize {
        self.inner.arg4()
    }

    pub fn arg5(&self) -> usize {
        self.inner.arg5()
    }

    pub fn arg6(&self) -> usize {
        self.inner.arg6()
    }
}
