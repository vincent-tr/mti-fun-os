use lazy_static::lazy_static;

use alloc::{collections::LinkedList, sync::Arc};
use spin::RwLock;

use super::Thread;

lazy_static! {
    pub static ref SCHEDULER: Scheduler = Scheduler::new();
}

#[derive(Debug)]
pub struct Scheduler {
    // TODO: Very basic for now
    ready_list: RwLock<LinkedList<Arc<Thread>>>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            ready_list: RwLock::new(LinkedList::new()),
        }
    }

    /// Add a new thread to the ready list
    pub fn add(&self, thread: Arc<Thread>) {
        let mut ready_list = self.ready_list.write();
        ready_list.push_back(thread);
    }

    /// Remove a thread from the ready list
    pub fn remove(&self, thread: &Arc<Thread>) {
        let mut ready_list = self.ready_list.write();

        // TODO: very inefficient
        let mut cursor = ready_list.cursor_back_mut();
        while let Some(item) = cursor.current() {
            if Arc::ptr_eq(item, &thread) {
                cursor.remove_current();
                return;
            }
        }

        // could not remove thread ?!
        panic!("thread {} not found in scheduler ready list", thread.id());
    }

    /// Decide which thread should be next executed, and pop it out of the ready list
    pub fn schedule(&self) -> Arc<Thread> {
        let mut ready_list = self.ready_list.write();
        ready_list.pop_front().expect("Ready list empty !")
    }
}
