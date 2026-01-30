use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU32, Ordering};

use libsyscalls::futex;

const UNLOCKED: u32 = 0;
const WRITER_BIT: u32 = 1 << 31;
const READER_MASK: u32 = !WRITER_BIT;
const MAX_READERS: u32 = READER_MASK;

/// A reader-writer lock
///
/// This type of lock allows multiple readers or at most one writer at any point in time.
/// The write portion of this lock typically allows modification of the underlying data
/// and the read portion of this lock typically allows for read-only access.
pub struct RwLock<T: ?Sized> {
    state: AtomicU32,
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
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this `RwLock`, returning the underlying data.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Locks this rwlock with shared read access, blocking the current thread
    /// until it can be acquired.
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        loop {
            // Try to increment reader count
            let state = self.state.load(Ordering::Relaxed);

            // Check if there's a writer or too many readers
            if (state & WRITER_BIT) != 0 || (state & READER_MASK) == MAX_READERS {
                self.read_contended();
                continue;
            }

            // Try to increment reader count
            if self
                .state
                .compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return RwLockReadGuard { lock: self };
            }
        }
    }

    #[cold]
    fn read_contended(&self) {
        let mut spin_count = 0;
        const MAX_SPINS: u32 = 100;

        loop {
            let state = self.state.load(Ordering::Relaxed);

            // If no writer and room for more readers, try again
            if (state & WRITER_BIT) == 0 && (state & READER_MASK) < MAX_READERS {
                return;
            }

            // Spin for a while
            if spin_count < MAX_SPINS {
                spin_count += 1;
                core::hint::spin_loop();
                continue;
            }

            // Wait on futex
            let state_ptr = self.state.as_ptr() as *const u32;
            let _ = futex::wait(unsafe { &*state_ptr }, state);
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

    /// Locks this rwlock with exclusive write access, blocking the current thread
    /// until it can be acquired.
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        // Try to acquire write lock
        if self
            .state
            .compare_exchange(UNLOCKED, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return RwLockWriteGuard { lock: self };
        }

        self.write_contended();

        RwLockWriteGuard { lock: self }
    }

    #[cold]
    fn write_contended(&self) {
        let mut spin_count = 0;
        const MAX_SPINS: u32 = 100;

        loop {
            // Try to acquire write lock
            if self
                .state
                .compare_exchange_weak(UNLOCKED, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }

            // Spin for a while
            if spin_count < MAX_SPINS {
                spin_count += 1;
                core::hint::spin_loop();
                continue;
            }

            // Wait on futex
            let state = self.state.load(Ordering::Relaxed);
            let state_ptr = self.state.as_ptr() as *const u32;
            let _ = futex::wait(unsafe { &*state_ptr }, state);
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

        // If this was the last reader, wake up waiting writers
        if readers == 1 {
            let state_ptr = self.state.as_ptr() as *const u32;
            let _ = futex::wake(unsafe { &*state_ptr }, 1);
        }
    }

    fn write_unlock(&self) {
        self.state.store(UNLOCKED, Ordering::Release);

        // Wake up all waiting threads (both readers and writers)
        let state_ptr = self.state.as_ptr() as *const u32;
        let _ = futex::wake(unsafe { &*state_ptr }, u32::MAX as usize);
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
