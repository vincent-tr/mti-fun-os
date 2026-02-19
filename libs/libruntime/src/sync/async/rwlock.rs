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
pub struct AsyncRwLock<T: ?Sized> {
    state: AtomicU32,
    read_waiters: SyncMutex<VecDeque<Waker>>,
    write_waiters: SyncMutex<VecDeque<Waker>>,
    data: UnsafeCell<T>,
}

/// RAII structure used to release the shared read access of a lock when dropped.
pub struct AsyncRwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a AsyncRwLock<T>,
}

/// RAII structure used to release the exclusive write access of a lock when dropped.
pub struct AsyncRwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a AsyncRwLock<T>,
}

unsafe impl<T: ?Sized + Send> Send for AsyncRwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for AsyncRwLock<T> {}

impl<T> AsyncRwLock<T> {
    /// Creates a new instance of an `AsyncRwLock<T>` which is unlocked.
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(UNLOCKED),
            read_waiters: SyncMutex::new(VecDeque::new()),
            write_waiters: SyncMutex::new(VecDeque::new()),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this `AsyncRwLock`, returning the underlying data.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> AsyncRwLock<T> {
    /// Locks this rwlock with shared read access, asynchronously waiting until it can be acquired.
    pub fn read(&self) -> AsyncRwLockReadFuture<'_, T> {
        AsyncRwLockReadFuture {
            lock: self,
            registered: false,
        }
    }

    /// Attempts to acquire this rwlock with shared read access.
    pub fn try_read(&self) -> Option<AsyncRwLockReadGuard<'_, T>> {
        let state = self.state.load(Ordering::Relaxed);

        if (state & WRITER_BIT) != 0 || (state & READER_MASK) == MAX_READERS {
            return None;
        }

        if self
            .state
            .compare_exchange(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(AsyncRwLockReadGuard { lock: self })
        } else {
            None
        }
    }

    /// Locks this rwlock with exclusive write access, asynchronously waiting until it can be acquired.
    pub fn write(&self) -> AsyncRwLockWriteFuture<'_, T> {
        AsyncRwLockWriteFuture {
            lock: self,
            registered: false,
        }
    }

    /// Attempts to lock this rwlock with exclusive write access.
    pub fn try_write(&self) -> Option<AsyncRwLockWriteGuard<'_, T>> {
        if self
            .state
            .compare_exchange(UNLOCKED, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(AsyncRwLockWriteGuard { lock: self })
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

/// Future returned by `AsyncRwLock::read()`.
pub struct AsyncRwLockReadFuture<'a, T: ?Sized> {
    lock: &'a AsyncRwLock<T>,
    registered: bool,
}

impl<'a, T: ?Sized> Future for AsyncRwLockReadFuture<'a, T> {
    type Output = AsyncRwLockReadGuard<'a, T>;

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
                    return Poll::Ready(AsyncRwLockReadGuard { lock: self.lock });
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

/// Future returned by `AsyncRwLock::write()`.
pub struct AsyncRwLockWriteFuture<'a, T: ?Sized> {
    lock: &'a AsyncRwLock<T>,
    registered: bool,
}

impl<'a, T: ?Sized> Future for AsyncRwLockWriteFuture<'a, T> {
    type Output = AsyncRwLockWriteGuard<'a, T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Try to acquire write lock
        if self
            .lock
            .state
            .compare_exchange(UNLOCKED, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Poll::Ready(AsyncRwLockWriteGuard { lock: self.lock });
        }

        // Can't acquire, register waker if not already registered
        if !self.registered {
            self.lock.write_waiters.lock().push_back(cx.waker().clone());
            self.registered = true;
        }

        Poll::Pending
    }
}

impl<T: ?Sized> Deref for AsyncRwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for AsyncRwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.read_unlock();
    }
}

impl<T: ?Sized> Deref for AsyncRwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for AsyncRwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for AsyncRwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.write_unlock();
    }
}
