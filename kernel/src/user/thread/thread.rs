use core::hash::{Hash, Hasher};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::{fmt, mem};

use alloc::sync::{Arc, Weak};
use hashbrown::HashSet;
use log::debug;
use spin::{Mutex, RwLock, RwLockReadGuard};
use syscalls::ThreadPriority;
use x86_64::registers::rflags::RFlags;

use crate::gdt::{USER_CODE_SELECTOR, USER_DATA_SELECTOR};
use crate::interrupts::{InterruptStack, USERLAND_RFLAGS};
use crate::memory::VirtAddr;
use crate::user::process::Process;

use super::wait_queue::WaitQueue;

/// Standalone function, so that Thread::new() can remain private
///
/// Note: Only Thread type is exported by thread module, not this function
pub fn new(
    id: u64,
    process: Arc<Process>,
    priority: ThreadPriority,
    thread_start: VirtAddr,
    stack_top: VirtAddr,
) -> Arc<Thread> {
    Thread::new(id, process, priority, thread_start, stack_top)
}

pub fn update_state(thread: &Arc<Thread>, new_state: ThreadState) {
    let mut state = thread.state.write();
    *state = new_state;
}

pub fn set_priority(thread: &Arc<Thread>, priority: ThreadPriority) {
    thread.set_priority(priority);
}

pub fn add_ticks(thread: &Arc<Thread>, ticks: usize) {
    thread.add_ticks(ticks);
}

pub unsafe fn save(thread: &Arc<Thread>) {
    let mut context = thread.context.lock();
    context.save(InterruptStack::current());
}

pub unsafe fn load(thread: &Arc<Thread>) {
    let context = thread.context.lock();
    context.load(InterruptStack::current());
}

/// Thread of execution
#[derive(Debug)]
pub struct Thread {
    id: u64,
    process: Arc<Process>,
    priority: AtomicU64,
    state: RwLock<ThreadState>,
    context: Mutex<ThreadContext>,
    ticks: AtomicUsize,
}

impl Thread {
    fn new(
        id: u64,
        process: Arc<Process>,
        priority: ThreadPriority,
        thread_start: VirtAddr,
        stack_top: VirtAddr,
    ) -> Arc<Self> {
        let thread = Arc::new(Self {
            id,
            process,
            priority: AtomicU64::new(priority as u64),
            state: RwLock::new(ThreadState::Ready), // a thread is ready by default
            context: Mutex::new(ThreadContext::new(thread_start, stack_top)),
            ticks: AtomicUsize::new(0),
        });

        debug!(
            "Thread {} created (pid={}, priority={:?}, thread_start={:?}, stack_top={:?})",
            thread.id,
            thread.process.id(),
            thread.priority(),
            thread_start,
            stack_top,
        );

        thread
    }

    /// Get the thread (global) identifier
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the process the threaad belong to
    pub fn process(&self) -> &Arc<Process> {
        &self.process
    }

    /// Get the state of the thread
    pub fn state(&self) -> RwLockReadGuard<ThreadState> {
        self.state.read()
    }

    /// Get the priority of the thread
    pub fn priority(&self) -> ThreadPriority {
        unsafe { mem::transmute(self.priority.load(Ordering::Relaxed)) }
    }

    /// Set the priority of the thread
    fn set_priority(&self, priority: ThreadPriority) {
        self.priority.store(priority as u64, Ordering::Relaxed);
    }

    /// Get the number of CPU ticks spent running the thread
    pub fn ticks(&self) -> usize {
        self.ticks.load(Ordering::Relaxed)
    }

    /// Add CPU ticks
    fn add_ticks(&self, ticks: usize) {
        self.ticks.fetch_add(ticks, Ordering::Relaxed);
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        debug!("Thread {} deleted (pid={})", self.id, self.process.id());
    }
}

/// State of a thread
#[derive(Debug)]
pub enum ThreadState {
    /// The thread is currently executing.
    ///
    /// When in kernel mode, this is the one that is currently configured as current, and on the interrupt stack
    Executing,

    /// This thread is ready to be scheduled
    Ready,

    /// This thread is sleeping, waiting for something
    Waiting(HashSet<WaitQueueRef>),

    /// This thread got an error (eg: page fault).
    ///
    /// It can be resumed after the error has been solved.
    Error(ThreadError),

    /// This thread has been terminated
    Terminated,
}

#[derive(Debug, Clone)]
pub struct WaitQueueRef(Weak<WaitQueue>);

impl WaitQueueRef {
    pub fn upgrade(&self) -> Option<Arc<WaitQueue>> {
        self.0.upgrade()
    }

    pub fn weak(&self) -> &Weak<WaitQueue> {
        &self.0
    }

    pub fn as_ptr(&self) -> *const WaitQueue {
        self.0.as_ptr()
    }
}

impl PartialEq for WaitQueueRef {
    fn eq(&self, other: &WaitQueueRef) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl Eq for WaitQueueRef {}

impl Hash for WaitQueueRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

impl From<&Arc<WaitQueue>> for WaitQueueRef {
    fn from(value: &Arc<WaitQueue>) -> Self {
        Self(Arc::downgrade(value))
    }
}

impl From<Weak<WaitQueue>> for WaitQueueRef {
    fn from(value: Weak<WaitQueue>) -> Self {
        Self(value)
    }
}

/// Error occured on a thread
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadError {
    PageFault,
}

impl ThreadState {
    pub fn is_executing(&self) -> bool {
        if let ThreadState::Executing = self {
            true
        } else {
            false
        }
    }

    pub fn is_ready(&self) -> bool {
        if let ThreadState::Ready = self {
            true
        } else {
            false
        }
    }

    pub fn is_waiting(&self) -> Option<&HashSet<WaitQueueRef>> {
        if let ThreadState::Waiting(wait_queues) = self {
            Some(wait_queues)
        } else {
            None
        }
    }

    pub fn is_error(&self) -> Option<ThreadError> {
        if let ThreadState::Error(error) = self {
            Some(*error)
        } else {
            None
        }
    }

    pub fn is_terminated(&self) -> bool {
        if let ThreadState::Terminated = self {
            true
        } else {
            false
        }
    }
}

/// Saved context of the thread.
struct ThreadContext {
    rax: usize,
    rcx: usize,
    rdx: usize,
    rbx: usize,
    rsi: usize,
    rdi: usize,
    /// Stack pointer
    rsp: VirtAddr,
    rbp: usize,
    r8: usize,
    r9: usize,
    r10: usize,
    r11: usize,
    r12: usize,
    r13: usize,
    r14: usize,
    r15: usize,

    /// Next instruction pointer
    instruction_pointer: VirtAddr,

    /// CPU flags
    cpu_flags: RFlags,
}

impl ThreadContext {
    pub const fn new(thread_start: VirtAddr, stack_top: VirtAddr) -> Self {
        Self {
            rax: 0,
            rcx: 0,
            rdx: 0,
            rbx: 0,
            rsi: 0,
            rdi: 0,
            rsp: stack_top,
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            instruction_pointer: thread_start,
            cpu_flags: USERLAND_RFLAGS,
        }
    }

    /// Save the interrupt stack into the thread context
    pub fn save(&mut self, interrupt_stack: &InterruptStack) {
        self.rax = interrupt_stack.scratch.rax;
        self.rcx = interrupt_stack.scratch.rcx;
        self.rdx = interrupt_stack.scratch.rdx;
        self.rbx = interrupt_stack.preserved.rbx;
        self.rsi = interrupt_stack.scratch.rsi;
        self.rdi = interrupt_stack.scratch.rdi;
        self.rsp = interrupt_stack.iret.stack_pointer;
        self.rbp = interrupt_stack.preserved.rbp;
        self.r8 = interrupt_stack.scratch.r8;
        self.r9 = interrupt_stack.scratch.r9;
        self.r10 = interrupt_stack.scratch.r10;
        self.r11 = interrupt_stack.scratch.r11;
        self.r12 = interrupt_stack.preserved.r12;
        self.r13 = interrupt_stack.preserved.r13;
        self.r14 = interrupt_stack.preserved.r14;
        self.r15 = interrupt_stack.preserved.r15;
        self.instruction_pointer = interrupt_stack.iret.instruction_pointer;
        self.cpu_flags = RFlags::from_bits_retain(interrupt_stack.iret.cpu_flags);
    }

    /// Load the thread context into the interrupt stack
    pub fn load(&self, interrupt_stack: &mut InterruptStack) {
        interrupt_stack.scratch.rax = self.rax;
        interrupt_stack.scratch.rcx = self.rcx;
        interrupt_stack.scratch.rdx = self.rdx;
        interrupt_stack.preserved.rbx = self.rbx;
        interrupt_stack.scratch.rsi = self.rsi;
        interrupt_stack.scratch.rdi = self.rdi;
        interrupt_stack.iret.stack_pointer = self.rsp;
        interrupt_stack.preserved.rbp = self.rbp;
        interrupt_stack.scratch.r8 = self.r8;
        interrupt_stack.scratch.r9 = self.r9;
        interrupt_stack.scratch.r10 = self.r10;
        interrupt_stack.scratch.r11 = self.r11;
        interrupt_stack.preserved.r12 = self.r12;
        interrupt_stack.preserved.r13 = self.r13;
        interrupt_stack.preserved.r14 = self.r14;
        interrupt_stack.preserved.r15 = self.r15;
        interrupt_stack.error_code = 0;
        interrupt_stack.iret.instruction_pointer = self.instruction_pointer;
        interrupt_stack.iret.cpu_flags = self.cpu_flags.bits();
        interrupt_stack.iret.code_segment = u64::from(USER_CODE_SELECTOR.0);
        interrupt_stack.iret.stack_segment = u64::from(USER_DATA_SELECTOR.0);
    }
}

impl fmt::Debug for ThreadContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadContext")
            .field("rax", &format_args!("{0:?} (0x{0:016X})", self.rax))
            .field("rcx", &format_args!("{0:?} (0x{0:016X})", self.rcx))
            .field("rdx", &format_args!("{0:?} (0x{0:016X})", self.rdx))
            .field("rbx", &format_args!("{0:?} (0x{0:016X})", self.rbx))
            .field("rsi", &format_args!("{0:?} (0x{0:016X})", self.rsi))
            .field("rdi", &format_args!("{0:?} (0x{0:016X})", self.rdi))
            .field("rsp", &format_args!("{0:?}) - stack pointer", self.rsp))
            .field("rbp", &format_args!("{0:?} (0x{0:016X})", self.rbp))
            .field("r8", &format_args!("{0:?} (0x{0:016X})", self.r8))
            .field("r9", &format_args!("{0:?} (0x{0:016X})", self.r9))
            .field("r10", &format_args!("{0:?} (0x{0:016X})", self.r10))
            .field("r11", &format_args!("{0:?} (0x{0:016X})", self.r11))
            .field("r12", &format_args!("{0:?} (0x{0:016X})", self.r12))
            .field("r13", &format_args!("{0:?} (0x{0:016X})", self.r13))
            .field("r14", &format_args!("{0:?} (0x{0:016X})", self.r14))
            .field("r15", &format_args!("{0:?} (0x{0:016X})", self.r15))
            .field(
                "instruction pointer",
                &format_args!("{0:?}", self.instruction_pointer),
            )
            .field("CPU flags pointer", &format_args!("{:?}", self.cpu_flags))
            .finish()
    }
}
