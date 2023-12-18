mod scheduler;
mod thread;
mod threads;
mod wait_queue;

use alloc::sync::Arc;
use hashbrown::HashSet;
use spin::RwLock;

pub use self::thread::{Thread, ThreadError, ThreadState};
use self::{threads::THREADS, wait_queue::WaitQueue};

use super::process::Process;
use crate::{
    memory::VirtAddr,
    user::thread::{
        scheduler::SCHEDULER,
        thread::{update_state, WaitQueueRef},
    },
};

pub fn create(process: Arc<Process>, thread_start: VirtAddr, stack_top: VirtAddr) -> Arc<Thread> {
    let thread = THREADS.create(process, thread_start, stack_top);

    assert!(thread.state().is_ready());
    SCHEDULER.add(thread.clone());

    thread
}

pub fn find(pid: u32) -> Option<Arc<Thread>> {
    THREADS.find(pid)
}

// Note: null before init
static CURRENT_THREAD: RwLock<Option<Arc<Thread>>> = RwLock::new(None);

/// Obtain the current executing thread
///
/// Note: will change after API call
pub fn current_thread() -> Arc<Thread> {
    let current = CURRENT_THREAD.read();
    current.as_ref().expect("No current thread").clone()
}

/// Add the thread to the specified wait queues
pub fn thread_sleep(thread: &Arc<Thread>, wait_queues: &[Arc<WaitQueue>]) {
    assert!(wait_queues.len() > 0);

    match *thread.state() {
        ThreadState::Executing => {
            switch_out(thread);
            switch_in(SCHEDULER.schedule());
        }
        ThreadState::Ready => {
            SCHEDULER.remove(thread);
        }
        ThreadState::Waiting(_) => {
            // Nothing to do
        }
        ThreadState::Error(_) | ThreadState::Terminated => {
            panic!("invalid thread state: {:?}", thread.state())
        }
    }

    let mut set: HashSet<WaitQueueRef> = HashSet::new();

    if let Some(existing) = thread.state().is_waiting() {
        for wait_queue_ref in existing {
            set.insert(wait_queue_ref.clone());
        }
    }

    for wait_queue in wait_queues {
        set.insert(wait_queue.into());
        wait_queue.add(thread.clone());
    }

    update_state(thread, ThreadState::Waiting(set));
}

/// Terminated the given thread
pub fn thread_terminate(thread: &Arc<Thread>) {
    assert!(!thread.state().is_terminated());

    if thread.state().is_executing() {
        switch_out(thread);
        switch_in(SCHEDULER.schedule());
    }

    update_state(thread, ThreadState::Terminated);
}

/// End of time slice: mark the current thread as ready, and schedule the next one
pub fn thread_next() {
    switch_out(&current_thread());
    switch_in(SCHEDULER.schedule());
}

/// Mark the given thread as errored
pub fn thread_error(thread: &Arc<Thread>, error: ThreadError) {
    assert!(!thread.state().is_terminated());
    assert!(!thread.state().is_error().is_some());

    if thread.state().is_executing() {
        switch_out(thread);
        switch_in(SCHEDULER.schedule());
    }

    update_state(thread, ThreadState::Error(error));
}

/// Wait up one thread from the wait queue
///
/// returns: true if OK, false if the wait_queue was empty
pub fn wait_queue_wake_one(wait_queue: &Arc<WaitQueue>) -> bool {
    let thread = match wait_queue.wake() {
        Some(thread) => thread,
        None => return false,
    };

    // Remove it from all other queues
    {
        let state = thread.state();
        let wait_queues = state.is_waiting().expect("thread not waiting");
        for wait_queue_ref in wait_queues {
            if Arc::as_ptr(wait_queue) != wait_queue_ref.as_ptr() {
                let wait_queue = wait_queue_ref.upgrade().expect("could not read wait queue");
                wait_queue.remove(&thread);
            }
        }
    }

    // Set it ready
    update_state(&thread, ThreadState::Ready);
    SCHEDULER.add(thread);

    return true;
}

/// Wait up all threads from the wait queue
pub fn wait_queue_wake_all(wait_queue: &Arc<WaitQueue>) {
    while wait_queue_wake_one(wait_queue) {}
}

fn switch_out(thread: &Arc<Thread>) {
    let mut current = CURRENT_THREAD.write();
    assert!(current.is_some() && Arc::ptr_eq(current.as_ref().unwrap(), thread));

    unsafe { thread::save(thread) };
    *current = None;
}

fn switch_in(thread: Arc<Thread>) {
    let mut current = CURRENT_THREAD.write();
    assert!(current.is_none());

    unsafe { thread::load(&thread) };
    *current = Some(thread);
}
