use alloc::sync::{Arc, Weak};

use crate::{interrupts::SyscallArgs, user::thread::Thread};

/// Syscall context
#[derive(Debug)]
pub struct Context {
    inner: SyscallArgs,
    owner: Weak<Thread>,
}

impl Context {
    pub fn from(inner: SyscallArgs, thread: &Arc<Thread>) -> Self {
        Self {
            inner,
            owner: Arc::downgrade(thread),
        }
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
