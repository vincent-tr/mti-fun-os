use alloc::sync::Arc;
use alloc::vec::Vec;
use core::{
    fmt,
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

use crate::sync::Mutex;

/// A one-shot notification that can be waited on by multiple tasks.
/// Once signaled, it remains signaled forever and cannot be reset.
#[derive(Clone)]
pub struct NotifyOnce {
    inner: Arc<NotifyOnceInner>,
}

struct NotifyOnceInner {
    signaled: AtomicBool,
    waiters: Mutex<Vec<Waker>>,
}

impl NotifyOnce {
    /// Create a new, unsignaled notification.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(NotifyOnceInner {
                signaled: AtomicBool::new(false),
                waiters: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Signal this notification, waking all waiters.
    /// This is idempotent - calling it multiple times has the same effect as calling it once.
    pub fn notify(&self) {
        if self.inner.signaled.load(Ordering::SeqCst) {
            return; // Already signaled, do nothing
        }

        self.inner.signaled.store(true, Ordering::SeqCst);

        // Wake all waiting tasks
        let mut waiters = self.inner.waiters.lock();
        for waker in waiters.drain(..) {
            waker.wake();
        }
    }

    /// Check if this notification has been signaled.
    pub fn is_notified(&self) -> bool {
        self.inner.signaled.load(Ordering::SeqCst)
    }

    /// Wait for this notification to be signaled.
    /// Returns a future that completes once the notification is signaled.
    pub fn wait(&self) -> NotifyOnceFuture {
        NotifyOnceFuture {
            notify: self.clone(),
        }
    }
}

impl fmt::Debug for NotifyOnce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotifyOnce")
            .field("signaled", &self.inner.signaled.load(Ordering::SeqCst))
            .finish()
    }
}

/// Future returned by `NotifyOnce::wait()`.
pub struct NotifyOnceFuture {
    notify: NotifyOnce,
}

impl Future for NotifyOnceFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.notify.inner.signaled.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }

        // Register this waker
        self.notify.inner.waiters.lock().push(cx.waker().clone());

        // Double-check after registration to avoid race condition
        if self.notify.inner.signaled.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl fmt::Debug for NotifyOnceFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotifyOnceFuture")
            .field("notify", &self.notify)
            .finish()
    }
}
