use core::fmt;

use alloc::sync::Arc;
use x86_64::registers::rflags::RFlags;

use crate::gdt::{USER_CODE_SELECTOR_INDEX, USER_DATA_SELECTOR_INDEX};
use crate::interrupts::{InterruptStack, USERLAND_RFLAGS};
use crate::memory::VirtAddr;

/// Standalone function, so that Thread::new() can remain private
///
/// Note: Only Thread type is exported by thread module, not this function
pub fn new(id: u32, thread_start: VirtAddr, stack_top: VirtAddr) -> Arc<Thread> {
    Thread::new(id, thread_start, stack_top)
}

/// Thread of execution
pub struct Thread {
    id: u32,
    state: ThreadState,
    context: ThreadContext,
}

impl Thread {
    fn new(id: u32, thread_start: VirtAddr, stack_top: VirtAddr) -> Arc<Self> {
        Arc::new(Self { 
            id, 
            state: ThreadState::Ready, // a thread is ready by default
            context: ThreadContext::new(thread_start, stack_top)
        })
    }

    /// Get the thread (global) identifier
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get the state of the thread
    pub fn state(&self) -> ThreadState {
        self.state
    }
}

/// State of a thread
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadState {
    /// The thread is currently executing.
    /// 
    /// When in kernel mode, this is the one that is currently configured as current, and on the interrupt stack
    Executing,

    /// This thread is ready to be scheduled
    Ready,

    /// This thread is sleeping, waiting for something
    Waiting,

    /// This thread got an error (eg: page fault).
    /// 
    /// It can be resumed after the error has been solved.
    Error,

    /// This thread has been terminated
    Terminated,
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
        interrupt_stack.iret.instruction_pointer = self.instruction_pointer;
        interrupt_stack.iret.cpu_flags = self.cpu_flags.bits();

        interrupt_stack.iret.code_segment = u64::from(USER_CODE_SELECTOR_INDEX);
        interrupt_stack.iret.stack_segment = u64::from(USER_DATA_SELECTOR_INDEX);
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
