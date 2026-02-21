use lazy_static::lazy_static;

use alloc::sync::Arc;
use spin::RwLock;
use syscalls::ThreadPriority;

use super::{Thread, queue::Queue};

lazy_static! {
    pub static ref SCHEDULER: Scheduler = Scheduler::new();
}

#[derive(Debug)]
pub struct Scheduler {
    ready_list: RwLock<[Queue; 7]>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            ready_list: RwLock::new([
                Queue::new(),
                Queue::new(),
                Queue::new(),
                Queue::new(),
                Queue::new(),
                Queue::new(),
                Queue::new(),
            ]),
        }
    }

    const fn index(priority: ThreadPriority) -> usize {
        // Note: reverse so that array starts with hightest priority
        ThreadPriority::TimeCritical as usize - priority as usize
    }

    /// Add a new thread to the ready list
    pub fn add(&self, thread: Arc<Thread>) {
        assert!(thread.state().is_ready());

        let mut ready_list = self.ready_list.write();
        let list = &mut ready_list[Self::index(thread.priority())];
        list.add(thread);
    }

    /// Remove a thread from the ready list
    pub fn remove(&self, thread: &Arc<Thread>) {
        let mut ready_list = self.ready_list.write();
        let list = &mut ready_list[Self::index(thread.priority())];
        assert!(
            list.remove(thread),
            "thread {} not found in scheduler ready list",
            thread.id()
        );
    }

    /// Decide which thread should be next executed, and pop it out of the ready list
    pub fn schedule(&self) -> Arc<Thread> {
        let mut ready_list = self.ready_list.write();

        // Hightest priority first
        for list in ready_list.iter_mut() {
            if let Some(thread) = list.pop() {
                return thread;
            }
        }

        panic!("Ready list empty !");
    }
}
