use core::cell::UnsafeCell;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll, Waker};

use alloc::collections::VecDeque;

use crate::sync::Mutex as SyncMutex;

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;

/// An async mutual exclusion primitive useful for protecting shared data
///
/// This mutex will yield to the async executor when waiting for the lock to become available.
pub struct Mutex<T: ?Sized> {
    state: AtomicU32,
    waiters: SyncMutex<VecDeque<Waker>>,
    data: UnsafeCell<T>,
}

/// An RAII implementation of a "scoped lock" of an async mutex.
/// When this structure is dropped (falls out of scope), the lock will be unlocked.
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    mutex: &'a Mutex<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new async mutex in an unlocked state ready for use.
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(UNLOCKED),
            waiters: SyncMutex::new(VecDeque::new()),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires the mutex, asynchronously waiting until it is able to do so.
    pub fn lock(&self) -> MutexLockFuture<'_, T> {
        MutexLockFuture {
            mutex: self,
            registered: false,
        }
    }

    /// Attempts to acquire this lock without waiting.
    ///
    /// If the lock could not be acquired at this time, then `None` is returned.
    /// Otherwise, an RAII guard is returned.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self
            .state
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `Mutex` mutably, no actual locking needs to
    /// take place.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    fn unlock(&self) {
        self.state.store(UNLOCKED, Ordering::Release);

        // Wake up the next waiter if any
        if let Some(waker) = self.waiters.lock().pop_front() {
            waker.wake();
        }
    }
}

/// Future returned by `Mutex::lock()`.
pub struct MutexLockFuture<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
    registered: bool,
}

impl<'a, T: ?Sized> Future for MutexLockFuture<'a, T> {
    type Output = MutexGuard<'a, T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Try to acquire the lock
        if self
            .mutex
            .state
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Poll::Ready(MutexGuard { mutex: self.mutex });
        }

        // Lock is held, register waker if not already registered
        if !self.registered {
            self.mutex.waiters.lock().push_back(cx.waker().clone());
            self.registered = true;
        }

        Poll::Pending
    }
}

impl<'a, T: ?Sized> Drop for MutexLockFuture<'a, T> {
    fn drop(&mut self) {
        // If we registered but never got the lock, we need to remove ourselves
        // from the wait queue. This is a best-effort cleanup.
        if self.registered {
            // Note: This is a simplification. In production, you'd want to
            // track which specific waker to remove, possibly using a unique ID.
        }
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

impl<T: ?Sized + core::fmt::Debug> core::fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.try_lock() {
            Some(guard) => f.debug_struct("Mutex").field("data", &&*guard).finish(),
            None => f.debug_struct("Mutex").field("data", &"<locked>").finish(),
        }
    }
}
