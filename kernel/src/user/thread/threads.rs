use alloc::vec::Vec;
use lazy_static::lazy_static;

use alloc::sync::Arc;

use crate::user::listener;
use crate::user::{id_gen::IdGen, process::Process, weak_map::WeakMap};

use crate::memory::VirtAddr;

use super::{thread, Thread, ThreadPriority};

lazy_static! {
    pub static ref THREADS: Threads = Threads::new();
}

#[derive(Debug)]
pub struct Threads {
    id_gen: IdGen,
    threads: WeakMap<u64, Thread>,
}

impl Threads {
    fn new() -> Self {
        Self {
            id_gen: IdGen::new(),
            threads: WeakMap::new(),
        }
    }

    /// Create a new thread
    pub fn create(
        &self,
        process: Arc<Process>,
        priority: ThreadPriority,
        thread_start: VirtAddr,
        stack_top: VirtAddr,
        arg: usize,
        tls: VirtAddr,
    ) -> Arc<Thread> {
        let id = self.id_gen.generate();
        let thread = thread::new(
            id,
            process.clone(),
            priority,
            thread_start,
            stack_top,
            arg,
            tls,
        );

        self.threads.insert(id, &thread);

        process.add_thread(&thread);

        listener::notify_thread(thread.id(), listener::ThreadEventType::Created);

        thread
    }

    /// Thread drop
    fn remove(&self, thread: &Thread) {
        self.threads.remove(thread.id());
    }

    /// Find a thread by its tid
    pub fn find(&self, tid: u64) -> Option<Arc<Thread>> {
        self.threads.find(&tid)
    }

    /// List tids
    pub fn list(&self) -> Vec<u64> {
        self.threads.keys()
    }
}

/// Reserved for thread drop
pub fn remove_thread(thread: &Thread) {
    THREADS.remove(thread)
}
