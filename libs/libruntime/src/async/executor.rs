use core::{
    fmt,
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{boxed::Box, collections::linked_list::LinkedList, sync::Arc};
use hashbrown::HashMap;
use lazy_static::lazy_static;

use crate::sync::{Mutex, MutexGuard};

use super::Reactor;

struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    waker: Waker,
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task").field("id", &self.id).finish()
    }
}

impl Task {
    pub fn new<F: Future<Output = ()> + Send + 'static>(task_id: TaskId, future: F) -> Self {
        Self {
            id: task_id,
            future: Box::pin(future),
            waker: Waker::from(Arc::new(TaskWaker::new(task_id))),
        }
    }

    pub fn poll(&mut self) -> Poll<()> {
        self.future
            .as_mut()
            .poll(&mut Context::from_waker(&self.waker))
    }
}

#[derive(Debug)]
struct TaskWaker {
    task_id: TaskId,
}

impl TaskWaker {
    pub fn new(task_id: TaskId) -> Self {
        Self { task_id }
    }
}

impl alloc::task::Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        Executor::get().wake(self.task_id);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        Executor::get().wake(self.task_id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TaskId(usize);

#[derive(Debug)]
struct TaskIdGenerator {
    next_id: AtomicUsize,
}

impl TaskIdGenerator {
    pub fn new() -> Self {
        Self {
            next_id: AtomicUsize::new(1),
        }
    }

    pub fn generate(&self) -> TaskId {
        TaskId(self.next_id.fetch_add(1, Ordering::SeqCst))
    }
}

/// Executor that runs tasks until completion.
#[derive(Debug)]
pub struct Executor {
    id_generator: TaskIdGenerator,
    tasks: HashMap<TaskId, Task>,
    ready_list: LinkedList<TaskId>,
}

impl Executor {
    /// Gets the global executor instance.
    pub fn get() -> MutexGuard<'static, Executor> {
        lazy_static! {
            static ref EXECUTOR: Mutex<Executor> = Mutex::new(Executor::new());
        }

        EXECUTOR.lock()
    }

    fn new() -> Self {
        Self {
            id_generator: TaskIdGenerator::new(),
            tasks: HashMap::new(),
            ready_list: LinkedList::new(),
        }
    }

    /// Spawns a new future onto the executor.
    pub fn spawn(&mut self, future: impl Future<Output = ()> + Send + 'static) {
        let task_id = self.id_generator.generate();
        let task = Task::new(task_id, future);
        self.tasks.insert(task_id, task);

        self.ready_list.push_back(task_id);
    }

    fn run_ready_once(&mut self) {
        let Some(task_id) = self.ready_list.pop_front() else {
            return;
        };

        let task = self.tasks.get_mut(&task_id).expect("Task not found");

        match task.poll() {
            Poll::Ready(()) => {
                // Task is done, remove it
                self.tasks.remove(&task_id);
            }
            Poll::Pending => {
                // Nothing to do, task is registered on the reactor and will be woken when ready
            }
        }
    }

    fn wake(&mut self, task_id: TaskId) {
        self.ready_list.push_back(task_id);
    }

    /// Run once, running a ready task if there is one, otherwise polling the reactor for ready waitable objects.
    pub fn run_once(&mut self) {
        if !self.ready_list.is_empty() {
            self.run_ready_once();
        } else {
            Reactor::get().poll();
        }
    }

    /// Checks if there are no tasks left in the executor.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}
