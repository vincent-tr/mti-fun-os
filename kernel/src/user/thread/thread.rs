use core::cell::RefCell;
use core::hash::{Hash, Hasher};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::{fmt, mem};

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use hashbrown::HashSet;
use log::debug;
use spin::{Mutex, RwLock, RwLockReadGuard};
use syscalls::Error;
pub use syscalls::ThreadPriority;
use x86_64::registers::rflags::RFlags;

use crate::gdt::{
    KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR, USER_CODE_SELECTOR, USER_DATA_SELECTOR,
};
use crate::interrupts::{
    Exception, InterruptStack, SyscallArgs, USERLAND_RFLAGS, tls_reg_read, tls_reg_write,
};
use crate::memory::{VirtAddr, is_userspace};
use crate::user::{
    error::invalid_argument,
    listener,
    process::{Process, process_remove_thread},
    syscalls::SyscallExecutor,
};

use super::{threads::remove_thread, wait_queue::WaitQueue};

/// Standalone function, so that Thread::new() can remain private
///
/// Note: Only Thread type is exported by thread module, not this function
pub fn new(
    id: u64,
    name: Option<&str>,
    process: Arc<Process>,
    privileged: bool,
    priority: ThreadPriority,
    thread_start: VirtAddr,
    stack_top: VirtAddr,
    arg: usize,
    tls: VirtAddr,
) -> Arc<Thread> {
    Thread::new(
        id,
        name,
        process,
        privileged,
        priority,
        thread_start,
        stack_top,
        arg,
        tls,
    )
}

pub fn update_state(thread: &Arc<Thread>, new_state: ThreadState) {
    let is_terminated = new_state.is_terminated();

    {
        let mut state = thread.state.write();
        *state = new_state;
    }

    if is_terminated {
        thread.process.thread_terminated();
    }
}

pub fn set_priority(thread: &Arc<Thread>, priority: ThreadPriority) {
    thread.set_priority(priority);
}

pub fn add_ticks(thread: &Arc<Thread>, ticks: usize) {
    thread.add_ticks(ticks);
}

// Unconditionaly clear the current syscall executor if any
pub fn syscall_clear(thread: &Arc<Thread>) {
    thread.syscall_clear();
}

pub unsafe fn save(thread: &Arc<Thread>) {
    let mut context = thread.context.lock();
    context.save();
}

pub unsafe fn load(thread: &Arc<Thread>) {
    let context = thread.context.lock();
    context.load(thread.privileged);
}

pub unsafe fn load_segments(thread: &Arc<Thread>) {
    ThreadContext::load_segments(thread.privileged);
}

/// Thread of execution
#[derive(Debug)]
pub struct Thread {
    id: u64,
    name: RwLock<Option<String>>,
    process: Arc<Process>,
    privileged: bool,
    priority: AtomicU64,
    state: RwLock<ThreadState>,
    context: Mutex<ThreadContext>,
    syscall: Mutex<Option<Arc<SyscallExecutor>>>,
    ticks: AtomicUsize,
}

impl Thread {
    fn new(
        id: u64,
        name: Option<&str>,
        process: Arc<Process>,
        privileged: bool,
        priority: ThreadPriority,
        thread_start: VirtAddr,
        stack_top: VirtAddr,
        arg: usize,
        tls: VirtAddr,
    ) -> Arc<Self> {
        let thread = Arc::new(Self {
            id,
            name: RwLock::new(name.map(String::from)),
            process,
            privileged,
            priority: AtomicU64::new(priority as u64),
            state: RwLock::new(ThreadState::Ready), // a thread is ready by default
            context: Mutex::new(ThreadContext::new(thread_start, stack_top, arg, tls)),
            syscall: Mutex::new(None),
            ticks: AtomicUsize::new(0),
        });

        debug!(
            "Thread {} created (name={}, pid={}, privileged={}, priority={:?}, thread_start={:?}, stack_top={:?})",
            thread.id,
            thread
                .name
                .read()
                .as_ref()
                .map_or("<None>", |str| str.as_str()),
            thread.process.id(),
            thread.privileged,
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

    /// Get the thread name
    pub fn name<'a>(&'a self) -> RwLockReadGuard<'a, Option<String>> {
        self.name.read()
    }

    /// Set the thread name
    pub fn set_name(&self, value: Option<&str>) {
        let mut name = self.name.write();
        *name = value.map(String::from);
    }

    /// Get the process the threaad belong to
    pub fn process(&self) -> &Arc<Process> {
        &self.process
    }

    /// Get the state of the thread
    pub fn state(&self) -> RwLockReadGuard<ThreadState> {
        self.state.read()
    }

    /// Get is the thread runs in privileged mode (ring0)
    pub fn privileged(&self) -> bool {
        self.privileged
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

    /// Enter syscall and add executor
    pub fn syscall_enter(&self, syscall: Arc<SyscallExecutor>) {
        let mut syscall_locked = self.syscall.lock();
        assert!(syscall_locked.is_none());
        *syscall_locked = Some(syscall);
    }

    /// Leave syscall and clear executor
    pub fn syscall_exit(&self, result: usize) {
        let mut syscall_locked = self.syscall.lock();
        assert!(syscall_locked.is_some());
        *syscall_locked = None;

        let is_executing = {
            let state = self.state();
            match *state {
                ThreadState::Executing => true,
                ThreadState::Ready => false,
                ThreadState::Waiting(_) | ThreadState::Error(_) | ThreadState::Terminated => {
                    panic!("Bad thread state to exit syscall: {:?}", *state)
                }
            }
        };

        if is_executing {
            // The context has not been saved, put it directly on the running interrupt stack
            SyscallArgs::set_current_result(result);
        } else {
            // Put the result on the thread context
            let mut context = self.context.lock();
            context.rax = result;
        }
    }

    // Unconditionaly clear the current syscall executor if any
    fn syscall_clear(&self) {
        let mut syscall_locked = self.syscall.lock();
        *syscall_locked = None;
    }

    /// Get the current syscall executor
    pub fn syscall(&self) -> Option<Arc<SyscallExecutor>> {
        let syscall_locked = self.syscall.lock();
        syscall_locked.clone()
    }

    /// Get the context of the thread, for userland
    pub fn get_user_context(&self, user: &mut syscalls::ThreadContext) {
        assert!(!self.state().is_executing());

        let context = self.context.lock();
        context.to_user(user);
    }

    pub fn update_user_context(
        &self,
        user: &[(syscalls::ThreadContextRegister, usize)],
    ) -> Result<(), Error> {
        assert!(!self.state().is_executing());

        let mut context = self.context.lock();

        for &(reg, value) in user {
            if !context.validate_from_user(reg, value) {
                return Err(invalid_argument());
            }
        }

        for &(reg, value) in user {
            context.from_user(reg, value);
        }

        Ok(())
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        remove_thread(self);
        process_remove_thread(self);

        debug!("Thread {} deleted (pid={})", self.id, self.process.id());
        listener::notify_thread(self, listener::ThreadEventType::Deleted);
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
    Waiting(WaitingData),

    /// This thread got an error (eg: page fault).
    ///
    /// It can be resumed after the error has been solved.
    Error(Exception),

    /// This thread has been terminated
    Terminated,
}

/// Data associated with the wait state of a thread
#[derive(Debug)]
pub struct WaitingData {
    context: RefCell<Option<Box<dyn WaitingContext>>>, // will be left empty after wakeup called
    wait_queues: HashSet<WaitQueueRef>,
}

unsafe impl Send for WaitingData {}
unsafe impl Sync for WaitingData {}

impl WaitingData {
    pub fn new<Context: WaitingContext + 'static>(
        context: Context,
        wait_queues: HashSet<WaitQueueRef>,
    ) -> Self {
        Self {
            context: RefCell::new(Some(Box::new(context))),
            wait_queues,
        }
    }

    pub fn wait_queues(&self) -> &HashSet<WaitQueueRef> {
        &self.wait_queues
    }

    pub fn take_context(&self) -> Box<dyn WaitingContext> {
        self.context
            .borrow_mut()
            .take()
            .expect("Cannot wakeup twice")
    }
}

/// Trait that represent the context of a waiting thread
///
/// Its implementation can hold the data that are required to resume thread execution
pub trait WaitingContext: fmt::Debug {
    /// Called by the thread management when the thread will resume.
    ///
    /// `wait_queue` is the wait queue that has been triggered to wake up the thread
    fn wakeup(self: Box<Self>, wait_queue: &Arc<WaitQueue>);
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

    pub fn is_waiting(&self) -> Option<&WaitingData> {
        if let ThreadState::Waiting(data) = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn is_error(&self) -> Option<Exception> {
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

    // FS base value (used for TLS)
    tls: VirtAddr,
}

impl ThreadContext {
    pub const fn new(
        thread_start: VirtAddr,
        stack_top: VirtAddr,
        arg: usize,
        tls: VirtAddr,
    ) -> Self {
        Self {
            rax: 0,
            rcx: 0,
            rdx: 0,
            rbx: 0,
            rsi: 0,
            rdi: arg,
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
            tls,
        }
    }

    /// Save the interrupt stack into the thread context
    pub unsafe fn save(&mut self) {
        let interrupt_stack = InterruptStack::current();

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
        self.cpu_flags = interrupt_stack.iret.cpu_flags;

        self.tls = tls_reg_read();
    }

    /// Load the thread context into the interrupt stack
    pub unsafe fn load(&self, privileged: bool) {
        let interrupt_stack = InterruptStack::current();

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
        interrupt_stack.iret.cpu_flags = self.cpu_flags;
        if privileged {
            interrupt_stack.iret.code_segment = KERNEL_CODE_SELECTOR;
            interrupt_stack.iret.stack_segment = KERNEL_DATA_SELECTOR;
        } else {
            interrupt_stack.iret.code_segment = USER_CODE_SELECTOR;
            interrupt_stack.iret.stack_segment = USER_DATA_SELECTOR;
        }

        tls_reg_write(self.tls);
    }

    /// Setup the interrupt stack with the right segments.
    pub unsafe fn load_segments(privileged: bool) {
        let interrupt_stack = InterruptStack::current();

        if privileged {
            interrupt_stack.iret.code_segment = KERNEL_CODE_SELECTOR;
            interrupt_stack.iret.stack_segment = KERNEL_DATA_SELECTOR;
        } else {
            interrupt_stack.iret.code_segment = USER_CODE_SELECTOR;
            interrupt_stack.iret.stack_segment = USER_DATA_SELECTOR;
        }
    }

    pub fn to_user(&self, user: &mut syscalls::ThreadContext) {
        user.rax = self.rax;
        user.rcx = self.rcx;
        user.rdx = self.rdx;
        user.rbx = self.rbx;
        user.rsi = self.rsi;
        user.rdi = self.rdi;
        user.rsp = self.rsp.as_u64() as usize;
        user.rbp = self.rbp;
        user.r8 = self.r8;
        user.r9 = self.r9;
        user.r10 = self.r10;
        user.r11 = self.r11;
        user.r12 = self.r12;
        user.r13 = self.r13;
        user.r14 = self.r14;
        user.r15 = self.r15;
        user.instruction_pointer = self.instruction_pointer.as_u64() as usize;
        user.cpu_flags = self.cpu_flags.bits() as usize;
        user.tls = self.tls.as_u64() as usize;
    }

    pub fn validate_from_user(&self, reg: syscalls::ThreadContextRegister, value: usize) -> bool {
        match reg {
            syscalls::ThreadContextRegister::Rsp => {
                let addr = VirtAddr::new(value as u64);
                is_userspace(addr)
            }
            syscalls::ThreadContextRegister::InstructionPointer => {
                let addr = VirtAddr::new(value as u64);
                is_userspace(addr)
            }
            syscalls::ThreadContextRegister::CpuFlags => {
                // Cannot change CPU Flags for now
                false
            }
            syscalls::ThreadContextRegister::TLS => {
                let addr = VirtAddr::new(value as u64);
                addr.is_null() || is_userspace(addr)
            }
            _ => true,
        }
    }

    pub fn from_user(&mut self, reg: syscalls::ThreadContextRegister, value: usize) {
        match reg {
            syscalls::ThreadContextRegister::Rax => {
                self.rax = value;
            }
            syscalls::ThreadContextRegister::Rcx => {
                self.rcx = value;
            }
            syscalls::ThreadContextRegister::Rdx => {
                self.rdx = value;
            }
            syscalls::ThreadContextRegister::Rbx => {
                self.rbx = value;
            }
            syscalls::ThreadContextRegister::Rsi => {
                self.rsi = value;
            }
            syscalls::ThreadContextRegister::Rdi => {
                self.rdi = value;
            }
            syscalls::ThreadContextRegister::Rsp => {
                self.rsp = VirtAddr::new(value as u64);
            }
            syscalls::ThreadContextRegister::Rbp => {
                self.rbp = value;
            }
            syscalls::ThreadContextRegister::R8 => {
                self.r8 = value;
            }
            syscalls::ThreadContextRegister::R9 => {
                self.r9 = value;
            }
            syscalls::ThreadContextRegister::R10 => {
                self.r10 = value;
            }
            syscalls::ThreadContextRegister::R11 => {
                self.r11 = value;
            }
            syscalls::ThreadContextRegister::R12 => {
                self.r12 = value;
            }
            syscalls::ThreadContextRegister::R13 => {
                self.r13 = value;
            }
            syscalls::ThreadContextRegister::R14 => {
                self.r14 = value;
            }
            syscalls::ThreadContextRegister::R15 => {
                self.r15 = value;
            }
            syscalls::ThreadContextRegister::InstructionPointer => {
                self.instruction_pointer = VirtAddr::new(value as u64);
            }
            syscalls::ThreadContextRegister::CpuFlags => {
                panic!("Cannot update CPU Flags from user");
            }
            syscalls::ThreadContextRegister::TLS => {
                self.tls = VirtAddr::new(value as u64);
            }
        }
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
