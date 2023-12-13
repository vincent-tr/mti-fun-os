use x86_64::registers::rflags::RFlags;

use crate::gdt::{USER_CODE_SELECTOR_INDEX, USER_DATA_SELECTOR_INDEX};
use crate::interrupts::{USERLAND_RFLAGS, InterruptStack};
use crate::memory::VirtAddr;

pub struct Thread {}

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
