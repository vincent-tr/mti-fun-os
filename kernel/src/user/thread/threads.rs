use lazy_static::lazy_static;

use alloc::sync::Arc;

use crate::user::{id_gen::IdGen, weak_map::WeakMap, process::Process};

use crate::memory::VirtAddr;

use super::{thread, Thread};

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
    pub fn create(&self, process: Arc<Process>, thread_start: VirtAddr, stack_top: VirtAddr) -> Arc<Thread> {
        let id = self.id_gen.generate();
        let thread = thread::new(id, process.clone(), thread_start, stack_top);

        self.threads.insert(id, &thread);

        process.add_thread(&thread);

        thread
    }

    /// Find a thread by its pid
    pub fn find(&self, pid: u64) -> Option<Arc<Thread>> {
        self.threads.find(pid)
    }
}
