use alloc::sync::Arc;
use spin::RwLock;

use super::{queue::Queue, Thread};

#[derive(Debug)]
pub struct WaitQueue {
    queue: RwLock<Queue>,
}

impl WaitQueue {
    pub fn new() -> Self {
        Self {
            queue: RwLock::new(Queue::new()),
        }
    }

    /// Add a new thread to this wait queue
    pub fn add(&self, thread: Arc<Thread>) {
        let mut queue = self.queue.write();
        queue.add(thread);
    }

    /// Remove a thread from the wait queue
    pub fn remove(&self, thread: &Arc<Thread>) {
        let mut queue = self.queue.write();
        assert!(
            queue.remove(thread),
            "thread {} not found in wait queue",
            thread.id()
        );
    }

    /// Wake up a thread to this wait queue
    pub fn wake(&self) -> Option<Arc<Thread>> {
        let mut queue = self.queue.write();
        queue.pop()
    }
}
