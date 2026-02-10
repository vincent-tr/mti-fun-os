use core::{
    ptr,
    sync::atomic::{AtomicBool, Ordering},
    task::Waker,
};

use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::{
    kobject,
    sync::{Mutex, MutexGuard},
};

#[derive(Debug)]
struct ReactorItem {
    waitable: *const dyn kobject::KWaitable,
    ready: *const AtomicBool,
    waker: Waker,
}

unsafe impl Send for ReactorItem {}

impl ReactorItem {
    pub fn new(waitable: &dyn kobject::KWaitable, ready: &AtomicBool, waker: Waker) -> Self {
        Self {
            waitable: waitable as *const dyn kobject::KWaitable as *const _,
            ready: ready as *const _,
            waker,
        }
    }

    pub fn set_ready(&self) {
        unsafe { &*self.ready }.store(true, Ordering::SeqCst);
        self.waker.wake_by_ref();
    }

    pub fn get_waitable(&self) -> &dyn kobject::KWaitable {
        unsafe { &*self.waitable }
    }
}

/// Reactor that manages waitable objects and their associated wakers.
#[derive(Debug)]
pub struct Reactor {
    waitables: Vec<ReactorItem>,
}

impl Reactor {
    /// Gets the global reactor instance.
    pub fn get() -> MutexGuard<'static, Reactor> {
        lazy_static! {
            static ref REACTOR: Mutex<Reactor> = Mutex::new(Reactor::new());
        }

        REACTOR.lock()
    }

    fn new() -> Self {
        Self {
            waitables: Vec::new(),
        }
    }

    /// Registers a waitable object with the reactor.
    pub fn register(
        &mut self,
        waitable: &dyn kobject::KWaitable,
        ready: &AtomicBool,
        waker: Waker,
    ) {
        self.waitables
            .push(ReactorItem::new(waitable, ready, waker));
    }

    /// Unregisters a waitable object from the reactor.
    pub fn unregister(&mut self, waitable: &dyn kobject::KWaitable) {
        self.waitables
            .retain(|item| !ptr::addr_eq(item.waitable as *const _, waitable as *const _));
    }

    /// Polls the reactor for ready waitable objects and wakes their associated wakers.
    pub fn poll(&mut self) {
        let mut waiter = kobject::Waiter::new(&[]);

        for item in &self.waitables {
            waiter.add(item.get_waitable());
        }

        waiter.wait().expect("Wait failed");

        for (index, item) in self.waitables.iter().enumerate() {
            if waiter.is_ready(index) {
                item.set_ready();
            }
        }
    }
}
