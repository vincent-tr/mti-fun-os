// From a userland POV, async is provided by kobject::KWaitable api

use alloc::{
    boxed::Box,
    collections::{linked_list::LinkedList, vec_deque::VecDeque},
    fmt,
    sync::Arc,
    task::Wake,
    vec::Vec,
};
use core::{
    future::Future,
    mem,
    ops::Range,
    pin::Pin,
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use hashbrown::{HashMap, HashSet, HashTable};
use lazy_static::lazy_static;

pub use crate::kobject;
use crate::sync::Mutex;

//// public API

pub async fn wait<Waitable: kobject::KWaitable>(waitable: &Waitable) {
    KWaitableFuture::new(waitable).await
}

//// public API END

#[derive(Debug)]
struct KWaitableFuture<'a, Waitable: kobject::KWaitable> {
    waitable: &'a Waitable,
    registered: AtomicBool,
    ready: AtomicBool,
}

impl<'a, Waitable: kobject::KWaitable> KWaitableFuture<'a, Waitable> {
    pub fn new(waitable: &'a Waitable) -> Self {
        Self {
            waitable,
            registered: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }

    fn register(&self, waker: Waker) {
        if self.registered.load(Ordering::SeqCst) {
            return;
        }

        REACTOR.lock().register(self.waitable, &self.ready, waker);
        self.registered.store(true, Ordering::SeqCst);
    }

    fn unregister(&self) {
        if !self.registered.load(Ordering::SeqCst) {
            return;
        }

        REACTOR.lock().unregister(self.waitable);
        self.registered.store(false, Ordering::SeqCst);
    }
}

impl<'a, Waitable: kobject::KWaitable> Drop for KWaitableFuture<'a, Waitable> {
    fn drop(&mut self) {
        self.unregister();
    }
}

impl<'a, Waitable: kobject::KWaitable> Future for KWaitableFuture<'a, Waitable> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.ready.load(Ordering::SeqCst) {
            self.unregister();

            Poll::Ready(())
        } else {
            self.register(cx.waker().clone());

            Poll::Pending
        }
    }
}

#[derive(Debug)]
pub struct ReactorItem {
    waitable: *const dyn kobject::KWaitable,
    ready: *const AtomicBool,
    waker: Waker,
}

unsafe impl Send for ReactorItem {}

impl ReactorItem {
    pub fn new(waitable: &dyn kobject::KWaitable, ready: &AtomicBool, waker: Waker) -> Self {
        Self {
            waitable: waitable as *const dyn kobject::KWaitable as *const _,
            ready: ready as *const _,
            waker,
        }
    }

    pub fn set_ready(&self) {
        unsafe { &*self.ready }.store(true, Ordering::SeqCst);
        self.waker.wake_by_ref();
    }

    pub fn get_waitable(&self) -> &dyn kobject::KWaitable {
        unsafe { &*self.waitable }
    }
}

lazy_static! {
    static ref REACTOR: Mutex<Reactor> = Mutex::new(Reactor::new());
}

#[derive(Debug)]
struct Reactor {
    waitables: Vec<ReactorItem>,
}

impl Reactor {
    pub fn new() -> Self {
        Self {
            waitables: Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        waitable: &dyn kobject::KWaitable,
        ready: &AtomicBool,
        waker: Waker,
    ) {
        self.waitables
            .push(ReactorItem::new(waitable, ready, waker));
    }

    pub fn unregister(&mut self, waitable: &dyn kobject::KWaitable) {
        self.waitables
            .retain(|item| !ptr::addr_eq(item.waitable as *const _, waitable as *const _));
    }

    pub fn poll(&mut self) {
        let mut waiter = kobject::Waiter::new(&[]);

        for item in &self.waitables {
            waiter.add(item.get_waitable());
        }

        waiter.wait().expect("Wait failed");

        for (index, item) in self.waitables.iter().enumerate() {
            if waiter.is_ready(index) {
                item.set_ready();
            }
        }
    }
}

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

    pub fn id(&self) -> TaskId {
        self.id
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
        EXECUTOR.lock().wake(self.task_id);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        EXECUTOR.lock().wake(self.task_id);
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

#[derive(Debug)]
struct Executor {
    id_generator: TaskIdGenerator,
    tasks: HashMap<TaskId, Task>,

    ready_list: LinkedList<TaskId>,
}

impl Executor {
    fn new() -> Self {
        Self {
            id_generator: TaskIdGenerator::new(),
            tasks: HashMap::new(),
            ready_list: LinkedList::new(),
        }
    }

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

    pub fn run_once(&mut self) {
        if !self.ready_list.is_empty() {
            self.run_ready_once();
        } else {
            REACTOR.lock().poll();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

lazy_static! {
    static ref EXECUTOR: Mutex<Executor> = Mutex::new(Executor::new());
}

/// Spawns a new future onto the executor.
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    EXECUTOR.lock().spawn(future);
}

/// Runs the executor until all tasks have completed.
pub fn block_on() {
    loop {
        EXECUTOR.lock().run_once();

        if EXECUTOR.lock().is_empty() {
            break;
        }
    }
}
