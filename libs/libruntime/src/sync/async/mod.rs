//! Async synchronization primitives
//!
//! This module provides async versions of synchronization primitives that work
//! with the async executor instead of blocking threads.

mod mutex;
mod rwlock;

pub use mutex::{Mutex, MutexGuard};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
