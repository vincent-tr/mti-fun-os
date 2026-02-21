use core::{
    mem, ptr,
    sync::atomic::{AtomicBool, Ordering},
    task::Waker,
};

use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::{kobject, sync::Mutex};

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
            waitable: unsafe {
                mem::transmute::<&dyn kobject::KWaitable, *const dyn kobject::KWaitable>(waitable)
            },
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
    waitables: Mutex<Vec<ReactorItem>>,
}

impl Reactor {
    /// Gets the global reactor instance.
    pub fn get() -> &'static Reactor {
        lazy_static! {
            static ref REACTOR: Reactor = Reactor::new();
        }

        &REACTOR
    }

    fn new() -> Self {
        Self {
            waitables: Mutex::new(Vec::new()),
        }
    }

    /// Registers a waitable object with the reactor.
    pub fn register(&self, waitable: &dyn kobject::KWaitable, ready: &AtomicBool, waker: Waker) {
        self.waitables
            .lock()
            .push(ReactorItem::new(waitable, ready, waker));
    }

    /// Unregisters a waitable object from the reactor.
    pub fn unregister(&self, waitable: &dyn kobject::KWaitable) {
        self.waitables
            .lock()
            .retain(|item| !ptr::addr_eq(item.waitable as *const _, waitable as *const _));
    }

    /// Polls the reactor for ready waitable objects and wakes their associated wakers.
    pub fn poll(&self) {
        let mut waiter = kobject::Waiter::new(&[]);

        // Keep locked during the poll, but we are monothreaded so that's OK.
        let waitables = self.waitables.lock();

        for item in waitables.iter() {
            waiter.add(item.get_waitable());
        }

        waiter.wait().expect("Wait failed");

        for (index, item) in waitables.iter().enumerate() {
            if waiter.is_ready(index) {
                item.set_ready();
            }
        }
    }
}
