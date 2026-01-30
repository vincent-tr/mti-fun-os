use alloc::{sync::Arc, vec::Vec};
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
    ///
    /// Note: do not use queue API directly, use thread::* functions instead
    pub fn add(&self, thread: Arc<Thread>) {
        let mut queue = self.queue.write();
        queue.add(thread);
    }

    /// Remove a thread from the wait queue
    ///
    /// Note: do not use queue API directly, use thread::* functions instead
    pub fn remove(&self, thread: &Arc<Thread>) {
        let mut queue = self.queue.write();
        assert!(
            queue.remove(thread),
            "thread {} not found in wait queue",
            thread.id()
        );
    }

    /// Wake up a thread to this wait queue
    ///
    /// Note: do not use queue API directly, use thread::* functions instead
    pub fn wake(&self) -> Option<Arc<Thread>> {
        let mut queue = self.queue.write();
        queue.pop()
    }

    /// Wake up all threads from this wait queue matching the given predicate
    ///
    /// Note: do not use queue API directly, use thread::* functions instead
    pub fn wake_all(&self, predicate: &dyn Fn(&Arc<Thread>) -> bool) -> Vec<Arc<Thread>> {
        let mut queue = self.queue.write();

        let threads = queue.list_predicate(predicate);

        for thread in &threads {
            queue.remove(&thread);
        }

        threads
    }

    /// Get the number of waiting threads in this queue
    pub fn len(&self) -> usize {
        let queue = self.queue.read();
        queue.len()
    }

    /// Test if this wait queue is empty
    pub fn empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for WaitQueue {
    fn drop(&mut self) {
        assert!(self.empty());
    }
}
