use super::handler::{InterruptStack, TrapOrigin};
use core::arch::naked_asm;
use core::fmt;
use memoffset::offset_of;
use x86_64::structures::gdt::SegmentSelector;

use crate::gdt::{self, USER_CODE_SELECTOR_INDEX, USER_DATA_SELECTOR_INDEX};
use crate::memory::VirtAddr;
use crate::user::execute_syscall;
use crate::user::thread::{thread_setup_sysret, userland_timer_begin, userland_timer_end};
use x86_64::registers::model_specific::{Efer, EferFlags};
use x86_64::registers::{
    model_specific::{LStar, SFMask, Star},
    rflags::RFlags,
};

use super::handler::ProcessorControlRegion;

pub fn init() {
    unsafe {
        Efer::update(|flags| {
            *flags |= EferFlags::SYSTEM_CALL_EXTENSIONS;
        });
    }

    const CS_SYSCALL: SegmentSelector = gdt::KERNEL_CODE_SELECTOR;
    const SS_SYSCALL: SegmentSelector = gdt::KERNEL_DATA_SELECTOR;
    const CS_SYSRET: SegmentSelector = gdt::USER_CODE_SELECTOR;
    const SS_SYSRET: SegmentSelector = gdt::USER_DATA_SELECTOR;

    Star::write(CS_SYSRET, SS_SYSRET, CS_SYSCALL, SS_SYSCALL)
        .expect("Could not setup 'star' register");

    let handler = VirtAddr::from_ptr(syscall_native_handler as *const ());
    LStar::write(handler);

    // Clear interrupts on syscall enter
    SFMask::write(RFlags::INTERRUPT_FLAG);
}

#[unsafe(naked)]
unsafe fn syscall_native_handler() {
    naked_asm!(concat!(
        "swapgs;",                    // Swap KGSBASE with GSBASE, allowing fast TSS access - https://www.felixcloutier.com/x86/swapgs - https://wiki.osdev.org/SWAPGS
        "mov gs:[{usp}], rsp;",       // Save userland stack pointer
        "mov rsp, gs:[{ksp}];",       // Load kernel stack pointer
        "push {user_ss};",            // Push fake userland SS (resembling iret frame)
        "push QWORD PTR gs:[{usp}];", // Push userland rsp
        "push r11;",                  // Push userland rflags
        "push {user_cs};",            // Push fake userland CS (resembling iret stack frame)
        "push rcx;",                  // Push userland return pointer
        "push 0;",                    // Fake error code
        "push {trap_origin};",        // Push trap origin
        "swapgs;",                    // Restore user GSBASE by swapping GSBASE and KGSBASE.

        "cld;",                       // Clear direction flag, required by ABI when running any Rust code in the kernel.

        push_scratch!(),
        push_preserved!(),

        // Call inner funtion
        "call {syscall_handler};",

        pop_preserved!(),
        pop_scratch!(),

        // Is it syscall or interrupt return?
        "cmp qword ptr [rsp + {trap_origin_offset}], {trap_origin_syscall};",
        "je 2f;",
        interrupt_return!(),
        "2:",
        syscall_return!(),
    ), 

    syscall_handler = sym syscall_handler,
    user_ss = const(USER_DATA_SELECTOR_INDEX),
    user_cs = const(USER_CODE_SELECTOR_INDEX),
    usp = const(offset_of!(ProcessorControlRegion, userland_stack_ptr_tmp)),
    ksp = const(offset_of!(ProcessorControlRegion, kernal_stack_ptr)),
    trap_origin = const(TrapOrigin::Syscall as usize),
    trap_origin_syscall = const(TrapOrigin::Syscall as usize),
    trap_origin_offset = const(offset_of!(InterruptStack, trap_origin)));
}

unsafe extern "C" fn syscall_handler() {
    userland_timer_end();

    let stack = unsafe { InterruptStack::current() };

    let n = stack.scratch.rax;
    let context = SyscallArgs::from_stack(stack);

    execute_syscall(n, context);

    thread_setup_sysret();

    userland_timer_begin();
}

/// Represent the context of a syscall
pub struct SyscallArgs {
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
}

impl fmt::Debug for SyscallArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyscallArgs")
            .field("arg1", &format_args!("{0:?} ({:#016x})", self.arg1))
            .field("arg2", &format_args!("{0:?} ({:#016x})", self.arg2))
            .field("arg3", &format_args!("{0:?} ({:#016x})", self.arg3))
            .field("arg4", &format_args!("{0:?} ({:#016x})", self.arg4))
            .field("arg5", &format_args!("{0:?} ({:#016x})", self.arg5))
            .field("arg6", &format_args!("{0:?} ({:#016x})", self.arg6))
            .finish()
    }
}

impl SyscallArgs {
    fn from_stack(stack: &InterruptStack) -> Self {
        Self {
            arg1: stack.scratch.rdi,
            arg2: stack.scratch.rsi,
            arg3: stack.scratch.rdx,
            arg4: stack.scratch.r10,
            arg5: stack.scratch.r8,
            arg6: stack.scratch.r9,
        }
    }

    pub fn arg1(&self) -> usize {
        self.arg1
    }

    pub fn arg2(&self) -> usize {
        self.arg2
    }

    pub fn arg3(&self) -> usize {
        self.arg3
    }

    pub fn arg4(&self) -> usize {
        self.arg4
    }

    pub fn arg5(&self) -> usize {
        self.arg5
    }

    pub fn arg6(&self) -> usize {
        self.arg6
    }

    /// Set return value on the current thread
    pub fn set_current_result(value: usize) {
        let stack = unsafe { InterruptStack::current() };
        stack.scratch.rax = value;
    }
}
