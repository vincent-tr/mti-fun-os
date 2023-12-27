use lazy_static::lazy_static;

use alloc::{collections::LinkedList, sync::Arc};
use spin::RwLock;
use syscalls::ThreadPriority;

use super::Thread;

lazy_static! {
    pub static ref SCHEDULER: Scheduler = Scheduler::new();
}

#[derive(Debug)]
pub struct Scheduler {
    ready_list: RwLock<[LinkedList<Arc<Thread>>; 7]>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            ready_list: RwLock::new([
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
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
        ready_list[Self::index(thread.priority())].push_back(thread);
    }

    /// Remove a thread from the ready list
    pub fn remove(&self, thread: &Arc<Thread>) {
        let mut ready_list = self.ready_list.write();

        // Note: very inefficient
        for list in ready_list.iter_mut() {
            let mut cursor = list.cursor_back_mut();
            while let Some(item) = cursor.current() {
                if Arc::ptr_eq(item, &thread) {
                    cursor.remove_current();
                    return;
                }
            }
        }

        // could not remove thread ?!
        panic!("thread {} not found in scheduler ready list", thread.id());
    }

    /// Decide which thread should be next executed, and pop it out of the ready list
    pub fn schedule(&self) -> Arc<Thread> {
        let mut ready_list = self.ready_list.write();

        // Hightest priority first
        for list in ready_list.iter_mut() {
            if let Some(thread) = list.pop_front() {
                return thread;
            }
        }

        panic!("Ready list empty !");
    }
}
