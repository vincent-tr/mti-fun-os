use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

use crate::kobject;

use super::Reactor;

/// Future that waits for a waitable object to become ready.
#[derive(Debug)]
pub struct KWaitableFuture<'a, Waitable: kobject::KWaitable> {
    waitable: &'a Waitable,
    registered: AtomicBool,
    ready: AtomicBool,
}

impl<'a, Waitable: kobject::KWaitable> KWaitableFuture<'a, Waitable> {
    /// Creates a new KWaitableFuture for the given waitable object.
    pub fn new(waitable: &'a Waitable) -> Self {
        Self {
            waitable,
            registered: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }

    fn register(&self, waker: Waker) {
        if self.registered.load(Ordering::SeqCst) {
            return;
        }

        Reactor::get().register(self.waitable, &self.ready, waker);
        self.registered.store(true, Ordering::SeqCst);
    }

    fn unregister(&self) {
        if !self.registered.load(Ordering::SeqCst) {
            return;
        }

        Reactor::get().unregister(self.waitable);
        self.registered.store(false, Ordering::SeqCst);
    }
}

impl<'a, Waitable: kobject::KWaitable> Drop for KWaitableFuture<'a, Waitable> {
    fn drop(&mut self) {
        self.unregister();
    }
}

impl<'a, Waitable: kobject::KWaitable> Future for KWaitableFuture<'a, Waitable> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.ready.load(Ordering::SeqCst) {
            self.unregister();

            Poll::Ready(())
        } else {
            self.register(cx.waker().clone());

            Poll::Pending
        }
    }
}
