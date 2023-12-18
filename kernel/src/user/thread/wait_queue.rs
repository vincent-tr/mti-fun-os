use alloc::{collections::LinkedList, sync::Arc};
use spin::RwLock;

use super::Thread;

#[derive(Debug)]
pub struct WaitQueue {
    queue: RwLock<LinkedList<Arc<Thread>>>,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            queue: RwLock::new(LinkedList::new()),
        }
    }

    /// Add a new thread to this wait queue
    pub fn add(&self, thread: Arc<Thread>) {
        let mut queue = self.queue.write();
        queue.push_back(thread);
    }

    /// Remove a thread from the wait queue
    pub fn remove(&self, thread: &Arc<Thread>) {
        let mut queue = self.queue.write();

        // TODO: very inefficient
        let mut cursor = queue.cursor_back_mut();
        while let Some(item) = cursor.current() {
            if Arc::ptr_eq(item, &thread) {
                cursor.remove_current();
                return;
            }
        }

        // could not remove thread ?!
        panic!("thread {} not found in scheduler ready list", thread.id());
    }

    /// Wake up a thread to this wait queue
    pub fn wake(&self) -> Option<Arc<Thread>> {
        let mut queue = self.queue.write();
        queue.pop_front()
    }
}
