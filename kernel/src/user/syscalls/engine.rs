use core::{
    cell::RefCell,
    fmt,
    future::{pending, Future},
    mem,
    pin::Pin,
    task,
};

use super::Context;
use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
    vec::Vec,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::trace;
use spin::RwLock;
use syscalls::{Error, SUCCESS};

use crate::{
    interrupts::SyscallArgs,
    user::{
        error::not_supported,
        thread::{self, thread_sleep, thread_terminate, Thread, WaitQueue},
    },
};

use super::SyscallNumber;

/// Type of a raw syscall handler (init handler)
pub trait SyscallRawHandler = Fn(SyscallArgs) + 'static;

/// Type of a syscall handler
pub trait SyscallHandler: (Fn(Context) -> Self::Future) + 'static {
    type Future: Future<Output = Result<(), Error>> + 'static;
}

impl<F, Fut> SyscallHandler for F
where
    F: (Fn(Context) -> Fut) + 'static,
    Fut: Future<Output = Result<(), Error>> + 'static,
{
    type Future = Fut;
}

struct Handlers {
    handlers: HashMap<SyscallNumber, Arc<dyn SyscallRawHandler>>,
}

unsafe impl Send for Handlers {}
unsafe impl Sync for Handlers {}

impl Handlers {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register<Handler: SyscallRawHandler>(
        &mut self,
        syscall_number: SyscallNumber,
        handler: Handler,
    ) {
        assert!(self
            .handlers
            .insert(syscall_number, Arc::from(handler))
            .is_none());
    }

    pub fn unregister(&mut self, syscall_number: SyscallNumber) {
        assert!(self.handlers.remove(&syscall_number).is_some());
    }

    pub fn get(&self, syscall_number: SyscallNumber) -> Option<Arc<dyn SyscallRawHandler>> {
        self.handlers.get(&syscall_number).cloned()
    }
}

lazy_static! {
    static ref HANDLERS: RwLock<Handlers> = RwLock::new(Handlers::new());
}

/// Execute a system call
pub fn execute_syscall(n: usize, context: SyscallArgs) {
    // If the number is not in struct we just won't get the key
    let syscall_number: SyscallNumber = unsafe { mem::transmute(n) };

    trace!("Syscall {syscall_number:?} {context:?}");

    // Do not keep the lock while executing, else we cannot register/unregister syscalls from a syscall
    let handler = {
        let handlers = HANDLERS.read();
        handlers.get(syscall_number)
    };

    if let Some(handler) = handler {
        handler(context);
    } else {
        SyscallArgs::set_current_result(not_supported() as usize);
    };
}

/// Register a new syscall handler
pub fn register_syscall_raw<Handler: SyscallRawHandler>(
    syscall_number: SyscallNumber,
    handler: Handler,
) {
    trace!("Add syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.register(syscall_number, handler);
}

/// Register a new syscall handler
pub fn register_syscall<Handler: SyscallHandler>(syscall_number: SyscallNumber, handler: Handler) {
    register_syscall_raw(syscall_number, move |inner: SyscallArgs| {
        let thread = thread::current_thread();
        let context = Context::from(inner, &thread);
        let future = handler(context);

        let executor = SyscallExecutor::new(thread.clone(), future);
        thread.syscall_enter(executor.clone());

        match executor.run_once() {
            task::Poll::Ready(result) => {
                // Syscall completed synchronously
                trace!("Syscall ret={result:?}");
                thread.syscall_exit(prepare_result(result));
            }
            task::Poll::Pending => {
                // Thread is either terminated or waiting, nothing to do
            }
        }
    });
}

/// Unregister a syscall handler
///
/// Used to remove the initial "init run" syscall
pub fn unregister_syscall(syscall_number: SyscallNumber) {
    trace!("Remove syscall {syscall_number:?}");
    let mut handlers = HANDLERS.write();
    handlers.unregister(syscall_number);
}

pub fn prepare_result(result: Result<(), Error>) -> usize {
    match result {
        Ok(_) => SUCCESS,
        Err(err) => err as usize,
    }
}

pub struct SyscallExecutor {
    thread: Weak<Thread>,
    future: RefCell<Pin<Box<dyn Future<Output = Result<(), Error>>>>>,
}

impl fmt::Debug for SyscallExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyscallExecutor")
            .field("thread", &self.thread)
            .field("future", &"<future>")
            .finish()
    }
}

unsafe impl Sync for SyscallExecutor {}
unsafe impl Send for SyscallExecutor {}

impl SyscallExecutor {
    pub fn new<TFuture: Future<Output = Result<(), Error>> + 'static>(
        thread: Arc<Thread>,
        future: TFuture,
    ) -> Arc<Self> {
        Arc::new(Self {
            thread: Arc::downgrade(&thread),
            future: RefCell::new(Box::pin(future)),
        })
    }

    pub fn run_once(self: &Arc<Self>) -> task::Poll<Result<(), Error>> {
        let waker = task::Waker::noop();
        let mut ctx = task::Context::from_waker(&waker);
        let mut borrowed = self.future.borrow_mut();
        let future = borrowed.as_mut();

        match future.poll(&mut ctx) {
            task::Poll::Ready(result) => task::Poll::Ready(result),
            task::Poll::Pending => {
                let thread = self
                    .thread
                    .upgrade()
                    .expect("Thread dropped while executing syscall");
                let state = thread.state();

                // Check the thread state. It can be Waiting (real async handling) or Terminated
                match *state {
                    thread::ThreadState::Terminated => {
                        // Thread terminated, we can drop the (never) future, nothing to do.
                    }
                    thread::ThreadState::Waiting(_) => {
                        // Thread in waiting state. the wake will poll it again, nothing to do right now.
                    }
                    _ => panic!("unexpected thread state {:?}", *state),
                };

                task::Poll::Pending
            }
        }
    }
}

/// Async API: make the thread sleep until one wait queue wake it up.
pub fn sleep(
    context: &Context,
    wait_queues: Vec<Arc<WaitQueue>>,
) -> impl Future<Output = Arc<WaitQueue>> {
    SleepFuture::new(context, wait_queues)
}

struct SleepFuture {
    thread: Weak<Thread>,
    state: SleepFutureState,
}

enum SleepFutureState {
    Created(Vec<Arc<WaitQueue>>),
    Sleeping(Arc<WaitResult>),
    Terminated,
}

impl SleepFuture {
    pub fn new(context: &Context, wait_queues: Vec<Arc<WaitQueue>>) -> Self {
        Self {
            thread: Arc::downgrade(&context.owner()),
            state: SleepFutureState::Created(wait_queues),
        }
    }
}

impl Future for SleepFuture {
    type Output = Arc<WaitQueue>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut self_future = self.as_mut();
        let thread = self_future
            .thread
            .upgrade()
            .expect("Thread dropped while sleeping");

        match &self_future.state {
            SleepFutureState::Created(wait_queues) => {
                // First step: sleep
                let wait_result = WaitResult::new();
                let wait_context = WaitCtx::new(self_future.thread.clone(), wait_result.clone());
                thread_sleep(&thread, wait_context, &wait_queues);

                self_future.state = SleepFutureState::Sleeping(wait_result);

                task::Poll::Pending
            }
            SleepFutureState::Sleeping(wait_result) => {
                let result = wait_result.take();

                self_future.state = SleepFutureState::Terminated;

                task::Poll::Ready(result)
            }
            SleepFutureState::Terminated => {
                panic!("Task terminated");
            }
        }
    }
}

/// Pointer on sleep result shared between context and future
#[derive(Debug)]
struct WaitResult(RefCell<Option<Arc<WaitQueue>>>);

impl WaitResult {
    pub fn new() -> Arc<Self> {
        Arc::new(Self(RefCell::new(None)))
    }

    pub fn set(&self, result: Arc<WaitQueue>) {
        let mut value = self.0.borrow_mut();
        *value = Some(result);
    }

    pub fn take(&self) -> Arc<WaitQueue> {
        let mut value = self.0.borrow_mut();
        value.take().expect("no value")
    }
}

#[derive(Debug)]
struct WaitCtx {
    thread: Weak<Thread>,
    result: Arc<WaitResult>,
}

impl WaitCtx {
    pub fn new(thread: Weak<Thread>, result: Arc<WaitResult>) -> Self {
        Self { thread, result }
    }
}

impl thread::WaitingContext for WaitCtx {
    fn wakeup(self: Box<Self>, wait_queue: &Arc<WaitQueue>) {
        self.result.set(wait_queue.clone());

        let thread = self
            .thread
            .upgrade()
            .expect("Thread dropped during syscall");
        let executor = thread.syscall().expect("Missing thread executor");

        match executor.run_once() {
            task::Poll::Ready(result) => {
                // Syscall terminated, set result
                trace!("Syscall ret={result:?}");
                thread.syscall_exit(prepare_result(result));
            }
            task::Poll::Pending => {
                // Thread is either terminated or waiting, nothing to do
            }
        }
    }
}

/// Async API: exit (never returns)
pub fn exit(context: &Context) -> impl Future<Output = !> {
    thread_terminate(&context.owner());

    return pending();
}
