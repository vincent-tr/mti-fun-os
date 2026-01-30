use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU32, Ordering};

use libsyscalls::futex;

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;
const CONTENDED: u32 = 2;

/// A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available using futexes.
pub struct Mutex<T: ?Sized> {
    state: AtomicU32,
    data: UnsafeCell<T>,
}

/// An RAII implementation of a "scoped lock" of a mutex.
/// When this structure is dropped (falls out of scope), the lock will be unlocked.
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    mutex: &'a Mutex<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(UNLOCKED),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires a mutex, blocking the current thread until it is able to do so.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        // Fast path: try to acquire the lock without contention
        if self
            .state
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return MutexGuard { mutex: self };
        }

        // Slow path: lock is contended
        self.lock_contended();

        MutexGuard { mutex: self }
    }

    #[cold]
    fn lock_contended(&self) {
        let mut spin_count = 0;
        const MAX_SPINS: u32 = 100;

        loop {
            // Try to acquire the lock
            let old = self.state.load(Ordering::Relaxed);

            if old == UNLOCKED
                && self
                    .state
                    .compare_exchange_weak(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
            {
                return;
            }

            // Spin for a while before blocking
            if spin_count < MAX_SPINS {
                spin_count += 1;
                core::hint::spin_loop();
                continue;
            }

            // Mark as contended if not already
            if old == LOCKED
                && self
                    .state
                    .compare_exchange_weak(LOCKED, CONTENDED, Ordering::Relaxed, Ordering::Relaxed)
                    .is_err()
            {
                continue;
            }

            // Wait on the futex
            let state_ptr = self.state.as_ptr() as *const u32;
            let _ = futex::wait(unsafe { &*state_ptr }, CONTENDED);
        }
    }

    /// Attempts to acquire this lock without blocking.
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
        // Fast path: if no contention, just unlock
        if self
            .state
            .compare_exchange(LOCKED, UNLOCKED, Ordering::Release, Ordering::Relaxed)
            .is_ok()
        {
            return;
        }

        // Slow path: there might be waiters
        self.unlock_contended();
    }

    #[cold]
    fn unlock_contended(&self) {
        // Set to unlocked
        self.state.store(UNLOCKED, Ordering::Release);

        // Wake up one waiter
        let state_ptr = self.state.as_ptr() as *const u32;
        let _ = futex::wake(unsafe { &*state_ptr }, 1);
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
