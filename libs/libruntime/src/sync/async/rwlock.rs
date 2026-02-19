use core::cell::UnsafeCell;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll, Waker};

use alloc::collections::VecDeque;

use crate::sync::Mutex as SyncMutex;

const UNLOCKED: u32 = 0;
const WRITER_BIT: u32 = 1 << 31;
const READER_MASK: u32 = !WRITER_BIT;
const MAX_READERS: u32 = READER_MASK;

/// An async reader-writer lock
///
/// This type of lock allows multiple readers or at most one writer at any point in time.
/// Tasks waiting for the lock will yield to the async executor instead of blocking.
pub struct RwLock<T: ?Sized> {
    state: AtomicU32,
    read_waiters: SyncMutex<VecDeque<Waker>>,
    write_waiters: SyncMutex<VecDeque<Waker>>,
    data: UnsafeCell<T>,
}

/// RAII structure used to release the shared read access of a lock when dropped.
pub struct RwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

/// RAII structure used to release the exclusive write access of a lock when dropped.
pub struct RwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// Creates a new instance of an `RwLock<T>` which is unlocked.
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(UNLOCKED),
            read_waiters: SyncMutex::new(VecDeque::new()),
            write_waiters: SyncMutex::new(VecDeque::new()),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this `RwLock`, returning the underlying data.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Locks this rwlock with shared read access, asynchronously waiting until it can be acquired.
    pub fn read(&self) -> RwLockReadFuture<'_, T> {
        RwLockReadFuture {
            lock: self,
            registered: false,
        }
    }

    /// Attempts to acquire this rwlock with shared read access.
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        let state = self.state.load(Ordering::Relaxed);

        if (state & WRITER_BIT) != 0 || (state & READER_MASK) == MAX_READERS {
            return None;
        }

        if self
            .state
            .compare_exchange(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(RwLockReadGuard { lock: self })
        } else {
            None
        }
    }

    /// Locks this rwlock with exclusive write access, asynchronously waiting until it can be acquired.
    pub fn write(&self) -> RwLockWriteFuture<'_, T> {
        RwLockWriteFuture {
            lock: self,
            registered: false,
        }
    }

    /// Attempts to lock this rwlock with exclusive write access.
    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        if self
            .state
            .compare_exchange(UNLOCKED, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(RwLockWriteGuard { lock: self })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying data.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    fn read_unlock(&self) {
        let old = self.state.fetch_sub(1, Ordering::Release);
        let readers = old & READER_MASK;

        // If this was the last reader, wake up one waiting writer
        if readers == 1 {
            if let Some(waker) = self.write_waiters.lock().pop_front() {
                waker.wake();
            }
        }
    }

    fn write_unlock(&self) {
        self.state.store(UNLOCKED, Ordering::Release);

        // Wake up all waiting readers first, then one writer
        let mut read_waiters = self.read_waiters.lock();
        while let Some(waker) = read_waiters.pop_front() {
            waker.wake();
        }
        drop(read_waiters);

        // If no readers were waiting, wake up one writer
        if let Some(waker) = self.write_waiters.lock().pop_front() {
            waker.wake();
        }
    }
}

/// Future returned by `RwLock::read()`.
pub struct RwLockReadFuture<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
    registered: bool,
}

impl<'a, T: ?Sized> Future for RwLockReadFuture<'a, T> {
    type Output = RwLockReadGuard<'a, T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let state = self.lock.state.load(Ordering::Relaxed);

            // Check if we can acquire read lock
            if (state & WRITER_BIT) == 0 && (state & READER_MASK) < MAX_READERS {
                if self
                    .lock
                    .state
                    .compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    return Poll::Ready(RwLockReadGuard { lock: self.lock });
                }
                // CAS failed, try again
                continue;
            }

            // Can't acquire, register waker if not already registered
            if !self.registered {
                self.lock.read_waiters.lock().push_back(cx.waker().clone());
                self.registered = true;
            }

            return Poll::Pending;
        }
    }
}

/// Future returned by `RwLock::write()`.
pub struct RwLockWriteFuture<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
    registered: bool,
}

impl<'a, T: ?Sized> Future for RwLockWriteFuture<'a, T> {
    type Output = RwLockWriteGuard<'a, T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Try to acquire write lock
        if self
            .lock
            .state
            .compare_exchange(UNLOCKED, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Poll::Ready(RwLockWriteGuard { lock: self.lock });
        }

        // Can't acquire, register waker if not already registered
        if !self.registered {
            self.lock.write_waiters.lock().push_back(cx.waker().clone());
            self.registered = true;
        }

        Poll::Pending
    }
}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.read_unlock();
    }
}

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.write_unlock();
    }
}

impl<T: ?Sized + core::fmt::Debug> core::fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.try_read() {
            Some(guard) => f.debug_struct("RwLock").field("data", &&*guard).finish(),
            None => f.debug_struct("RwLock").field("data", &"<locked>").finish(),
        }
    }
}
