use core::{hash::Hash, ptr::NonNull};
use lazy_static::lazy_static;

use hashbrown::HashSet;
use spin::RwLock;

use super::{timer::timer_tick, Timer};

lazy_static! {
    pub static ref TIMERS: Timers = Timers::new();
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct TimerPtr(NonNull<Timer>);

impl TimerPtr {
    pub fn new(value: &Timer) -> Self {
        Self(NonNull::from(value))
    }
}

unsafe impl Send for TimerPtr {}
unsafe impl Sync for TimerPtr {}

#[derive(Debug)]
pub struct Timers(RwLock<HashSet<TimerPtr>>);

impl Timers {
    pub fn new() -> Self {
        Self(RwLock::new(HashSet::new()))
    }

    pub fn add(&self, timer: &Timer) {
        let mut set = self.0.write();

        set.insert(TimerPtr::new(timer));
    }

    pub fn remove(&self, timer: &Timer) {
        let mut set = self.0.write();

        set.remove(&TimerPtr::new(timer));
    }

    pub fn tick(&self, now: u64) {
        let set = self.0.read();
        for timer_ptr in set.iter() {
            let timer = unsafe { timer_ptr.0.as_ref() };
            timer_tick(timer, now);
        }
    }
}
