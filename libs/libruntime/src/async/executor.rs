use core::{
    fmt,
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{boxed::Box, collections::linked_list::LinkedList, sync::Arc};
use hashbrown::HashMap;
use lazy_static::lazy_static;

use crate::sync::{Mutex, RwLock};

use super::Reactor;

struct Task {
    id: TaskId,
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    waker: Waker,
    completed: AtomicBool,
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task").field("id", &self.id).finish()
    }
}

impl Task {
    pub fn new(task_id: TaskId, future: impl Future<Output = ()> + Send + 'static) -> Self {
        Self {
            id: task_id,
            future: Mutex::new(Box::pin(future)),
            waker: Waker::from(Arc::new(TaskWaker::new(task_id))),
            completed: AtomicBool::new(false),
        }
    }

    pub fn poll(&self) -> Poll<()> {
        if self.completed.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }

        match self
            .future
            .lock()
            .as_mut()
            .poll(&mut Context::from_waker(&self.waker))
        {
            Poll::Ready(()) => {
                self.completed.store(true, Ordering::SeqCst);
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
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
    tasks: RwLock<HashMap<TaskId, Arc<Task>>>,
    ready_list: Mutex<LinkedList<TaskId>>,
}

impl Executor {
    /// Gets the global executor instance.
    pub fn get() -> &'static Executor {
        lazy_static! {
            static ref EXECUTOR: Executor = Executor::new();
        }

        &EXECUTOR
    }

    fn new() -> Self {
        Self {
            id_generator: TaskIdGenerator::new(),
            tasks: RwLock::new(HashMap::new()),
            ready_list: Mutex::new(LinkedList::new()),
        }
    }

    /// Spawns a new future onto the executor.
    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> JoinHandle {
        let task_id = self.id_generator.generate();
        let task = Arc::new(Task::new(task_id, future));
        self.tasks.write().insert(task_id, task.clone());

        self.ready_list.lock().push_back(task_id);

        JoinHandle { task }
    }

    fn run_ready_once(&self) -> bool {
        let Some(task_id) = self.ready_list.lock().pop_front() else {
            return false;
        };

        let task = self
            .tasks
            .read()
            .get(&task_id)
            .expect("Task not found")
            .clone();

        match task.poll() {
            Poll::Ready(()) => {
                // Task is done, remove it
                self.tasks.write().remove(&task_id);
            }
            Poll::Pending => {
                // Nothing to do, task is registered on the reactor and will be woken when ready
            }
        }

        true
    }

    fn wake(&self, task_id: TaskId) {
        self.ready_list.lock().push_back(task_id);
    }

    /// Run once, running a ready task if there is one, otherwise polling the reactor for ready waitable objects.
    pub fn run_once(&self) {
        // try to run a ready task first, if there is one
        if !self.run_ready_once() {
            // No ready tasks, poll the reactor for waitable objects
            Reactor::get().poll();
        }
    }

    /// Checks if there are no tasks left in the executor.
    pub fn is_empty(&self) -> bool {
        self.tasks.read().is_empty()
    }
}

/// Handle to a spawned task that can be awaited.
#[derive(Debug, Clone)]
pub struct JoinHandle {
    task: Arc<Task>,
}

impl Future for JoinHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.task.completed.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }

        self.task.poll()
    }
}
