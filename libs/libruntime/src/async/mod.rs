// From a userland POV, async is provided by kobject::KWaitable api

use alloc::{
    boxed::Box,
    collections::{linked_list::LinkedList, vec_deque::VecDeque},
    task::Wake,
    vec::Vec,
};
use core::{
    future::Future,
    mem,
    ops::Range,
    pin::Pin,
    ptr,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use hashbrown::{HashMap, HashSet, HashTable};
use lazy_static::lazy_static;

pub use crate::kobject;
use crate::sync::Mutex;

//// public API

pub async fn wait<Waitable: kobject::KWaitable>(waitable: &Waitable) {
    KWaitableFuture::new(waitable).await
}

//// public API END

#[derive(Debug)]
struct KWaitableFuture<'a, Waitable: kobject::KWaitable> {
    waitable: &'a Waitable,
    registered: AtomicBool,
    ready: AtomicBool,
}

impl<'a, Waitable: kobject::KWaitable> KWaitableFuture<'a, Waitable> {
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

        REACTOR.lock().register(self.waitable, &self.ready, waker);
        self.registered.store(true, Ordering::SeqCst);
    }

    fn unregister(&self) {
        if !self.registered.load(Ordering::SeqCst) {
            return;
        }

        REACTOR.lock().unregister(self.waitable);
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

#[derive(Debug)]
pub struct ReactorItem {
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

lazy_static! {
    static ref REACTOR: Mutex<Reactor> = Mutex::new(Reactor::new());
}

#[derive(Debug)]
struct Reactor {
    waitables: Vec<ReactorItem>,
}

impl Reactor {
    pub fn new() -> Self {
        Self {
            waitables: Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        waitable: &dyn kobject::KWaitable,
        ready: &AtomicBool,
        waker: Waker,
    ) {
        self.waitables
            .push(ReactorItem::new(waitable, ready, waker));
    }

    pub fn unregister(&mut self, waitable: &dyn kobject::KWaitable) {
        self.waitables
            .retain(|item| !ptr::addr_eq(item.waitable as *const _, waitable as *const _));
    }

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
