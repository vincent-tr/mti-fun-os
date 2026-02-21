mod queue;
mod scheduler;
mod thread;
mod threads;
mod wait_queue;

use alloc::{sync::Arc, vec::Vec};
use hashbrown::HashSet;
use log::debug;
use spin::RwLock;

use self::{
    scheduler::SCHEDULER,
    thread::{WaitQueueRef, WaitingData, add_ticks, load_segments, syscall_clear, update_state},
    threads::THREADS,
};
pub use self::{
    thread::{Thread, ThreadPriority, ThreadState, WaitingContext},
    wait_queue::WaitQueue,
};

use super::process::Process;
use crate::{interrupts::Exception, memory::VirtAddr, user::listener};

pub fn create(
    name: Option<&str>,
    process: Arc<Process>,
    privileged: bool,
    priority: ThreadPriority,
    thread_start: VirtAddr,
    stack_top: VirtAddr,
    arg: usize,
    tls: VirtAddr,
) -> Arc<Thread> {
    let thread = THREADS.create(
        name,
        process,
        privileged,
        priority,
        thread_start,
        stack_top,
        arg,
        tls,
    );

    assert!(thread.state().is_ready());
    SCHEDULER.add(thread.clone());

    thread
}

pub fn find(tid: u64) -> Option<Arc<Thread>> {
    THREADS.find(tid)
}

pub fn list() -> Vec<u64> {
    THREADS.list()
}

/// Setup initial thread
///
/// Pick it from the scheduler queue, and make it as current
///
pub fn initial_setup_thread() {
    let new_thread = SCHEDULER.schedule();
    let new_process = new_thread.process();
    let address_space = new_process.address_space().write();
    unsafe { crate::memory::set_current_address_space(&address_space) };

    unsafe { thread::load(&new_thread) };

    let mut current = CURRENT_THREAD.write();
    update_state(&new_thread, ThreadState::Executing);
    *current = Some(new_thread.clone());
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

fn context_switch(new_thread: Arc<Thread>) {
    assert!(new_thread.state().is_ready());

    let mut current = CURRENT_THREAD.write();
    let old_thread = current.as_ref().expect("no current thread");

    if Arc::ptr_eq(old_thread, &new_thread) {
        // Same thread, nothing to do
        update_state(&new_thread, ThreadState::Executing);
        return;
    }

    unsafe { thread::save(old_thread) };

    let new_process = new_thread.process();
    if !Arc::ptr_eq(old_thread.process(), new_process) {
        let address_space = new_process.address_space().read();
        unsafe { crate::memory::set_current_address_space(&address_space) };
    }

    unsafe { thread::load(&new_thread) };

    update_state(&new_thread, ThreadState::Executing);
    *current = Some(new_thread);
}

/// Add the thread to the specified wait queues
pub fn thread_sleep<Context: WaitingContext + 'static>(
    thread: &Arc<Thread>,
    context: Context,
    wait_queues: &[Arc<WaitQueue>],
) {
    assert!(wait_queues.len() > 0);

    match *thread.state() {
        ThreadState::Executing => {
            // Note: syscall must be async
            context_switch(SCHEDULER.schedule());
        }
        ThreadState::Ready => {
            SCHEDULER.remove(thread);
        }
        _ => {
            panic!("invalid thread state: {:?}", thread.state())
        }
    }

    let mut set: HashSet<WaitQueueRef> = HashSet::new();

    for wait_queue in wait_queues {
        set.insert(wait_queue.into());
        wait_queue.add(thread.clone());
    }

    update_state(thread, ThreadState::Waiting(WaitingData::new(context, set)));
}

/// Terminated the given thread
pub fn thread_terminate(thread: &Arc<Thread>) {
    match &*thread.state() {
        ThreadState::Executing => {
            context_switch(SCHEDULER.schedule());
        }
        ThreadState::Ready => {
            SCHEDULER.remove(thread);
        }
        ThreadState::Waiting(data) => {
            // Remove it from all queues
            for wait_queue_ref in data.wait_queues() {
                let wait_queue = wait_queue_ref.upgrade().expect("could not read wait queue");
                wait_queue.remove(&thread);
            }
        }
        ThreadState::Error(_) => {
            // Nothing to do, thread is already in no queue
        }
        ThreadState::Terminated => {
            panic!("invalid thread state: {:?}", thread.state())
        }
    }

    syscall_clear(thread);
    update_state(thread, ThreadState::Terminated);
    listener::notify_thread(&thread, listener::ThreadEventType::Terminated);
}

/// End of time slice: mark the current thread as ready, and schedule the next one
pub fn thread_next() {
    // Add the current thread is the ready list and trigger the scheduler.
    // Note: the same thread may pop out if there is only one ready/executing thread
    let old_thread = current_thread();
    update_state(&old_thread, ThreadState::Ready);
    SCHEDULER.add(old_thread);

    context_switch(SCHEDULER.schedule());
}

/// Triggered from exception handler: mark the current thread as errored
pub fn thread_error(error: Exception) {
    let thread = current_thread();

    debug!("Thread {} error: {:?}", thread.id(), error);

    context_switch(SCHEDULER.schedule());

    update_state(&thread, ThreadState::Error(error));
    listener::notify_thread(&thread, listener::ThreadEventType::Error);
}

/// Resume the given errored thread
pub fn thread_resume(thread: &Arc<Thread>) {
    assert!(thread.state().is_error().is_some());

    // Set it ready
    update_state(&thread, ThreadState::Ready);
    SCHEDULER.add(thread.clone());

    listener::notify_thread(&thread, listener::ThreadEventType::Resumed);
}

/// Wait up one thread from the wait queue
///
/// returns: true if OK, false if the wait_queue was empty
pub fn wait_queue_wake_one(wait_queue: &Arc<WaitQueue>) -> bool {
    let thread = match wait_queue.wake() {
        Some(thread) => thread,
        None => return false,
    };

    wake_thread(thread, wait_queue);

    return true;
}

/// Wait up all threads from the wait queue
pub fn wait_queue_wake_all(
    wait_queue: &Arc<WaitQueue>,
    predicate: &dyn Fn(&Arc<Thread>) -> bool,
) -> usize {
    let threads = wait_queue.wake_all(predicate);
    let count = threads.len();

    for thread in threads {
        wake_thread(thread, wait_queue);
    }

    count
}

fn wake_thread(thread: Arc<Thread>, wait_queue: &Arc<WaitQueue>) {
    let wait_context = {
        let state = thread.state();
        let data = state.is_waiting().expect("thread not waiting");

        // Remove it from all other queues
        for wait_queue_ref in data.wait_queues() {
            if Arc::as_ptr(wait_queue) != wait_queue_ref.as_ptr() {
                let wait_queue = wait_queue_ref.upgrade().expect("could not read wait queue");
                wait_queue.remove(&thread);
            }
        }

        data.take_context()
    };

    // Set it ready
    update_state(&thread, ThreadState::Ready);
    SCHEDULER.add(thread);

    // Resume it
    wait_context.wakeup(wait_queue);
}

/// Set the thread priority
pub fn thread_set_priority(thread: &Arc<Thread>, priority: ThreadPriority) {
    // re-queue the thread so that the new priority is applied
    // Note: cannot change priority while thread is in queue, else we will not be able to remove it after
    if thread.state().is_ready() {
        SCHEDULER.remove(thread);
    }

    thread::set_priority(thread, priority);

    if thread.state().is_ready() {
        SCHEDULER.add(thread.clone());
    }
}

static mut USERLAND_TIMER_BEGIN_TICKS: usize = 0;

/// Call just before switch to userland
pub fn userland_timer_begin() {
    unsafe {
        USERLAND_TIMER_BEGIN_TICKS = core::arch::x86_64::_rdtsc() as usize;
    }
}

/// Call just after switch from userland
pub fn userland_timer_end() {
    let (begin, end) = unsafe {
        let begin = USERLAND_TIMER_BEGIN_TICKS;
        if begin == 0 {
            // This is first syscall enter, no begin has been called.
            // Nothing to count
            return;
        }

        USERLAND_TIMER_BEGIN_TICKS = 0;

        let end = core::arch::x86_64::_rdtsc() as usize;

        (begin, end)
    };

    add_ticks(&current_thread(), end - begin);
}

pub struct UserlandTimerInterruptScope {}

impl UserlandTimerInterruptScope {
    pub fn new() -> Self {
        userland_timer_end();
        Self {}
    }
}

impl Drop for UserlandTimerInterruptScope {
    fn drop(&mut self) {
        userland_timer_begin();
    }
}

/// Setup the interrupt stack with the right segments.
///
/// This is mandatory to detect if we should return to ring0
pub fn thread_setup_sysret() {
    let thread = current_thread();

    unsafe {
        load_segments(&thread);
    }
}
