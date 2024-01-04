use super::handler::InterruptStack;
use core::arch::asm;
use core::fmt;
use memoffset::offset_of;

use crate::gdt::{USER_CODE_SELECTOR_INDEX, USER_DATA_SELECTOR_INDEX};
use crate::memory::VirtAddr;
use crate::user::execute_syscall;
use crate::user::thread::{userland_timer_begin, userland_timer_end};
use x86_64::registers::model_specific::{Efer, EferFlags};
use x86_64::registers::{
    model_specific::{LStar, SFMask, Star},
    rflags::RFlags,
};

use super::handler::ProcessorControlRegion;

use crate::gdt;

pub fn init() {
    unsafe {
        Efer::update(|flags| {
            *flags |= EferFlags::SYSTEM_CALL_EXTENSIONS;
        });
    }

    let cs_syscall = gdt::kernel_code_selector();
    let ss_syscall = gdt::kernel_data_selector();
    let cs_sysret = gdt::user_code_selector();
    let ss_sysret = gdt::user_data_selector();

    Star::write(cs_sysret, ss_sysret, cs_syscall, ss_syscall)
        .expect("Could not setup 'star' register");

    let handler = VirtAddr::from_ptr(syscall_native_handler as *const ());
    LStar::write(handler);

    // Clear interrupts on syscall enter
    SFMask::write(RFlags::INTERRUPT_FLAG);
}

#[naked]
#[allow(undefined_naked_function_abi)]
unsafe fn syscall_native_handler() {
    unsafe {
        asm!(concat!(
            "swapgs;",                    // Swap KGSBASE with GSBASE, allowing fast TSS access - https://www.felixcloutier.com/x86/swapgs - https://wiki.osdev.org/SWAPGS
            "mov gs:[{usp}], rsp;",       // Save userland stack pointer
            "mov rsp, gs:[{ksp}];",       // Load kernel stack pointer
            "push QWORD PTR {ss_sel};",   // Push fake userland SS (resembling iret frame)
            "push QWORD PTR gs:[{usp}];", // Push userland rsp
            "push r11;",                  // Push userland rflags
            "push QWORD PTR {cs_sel};",   // Push fake userland CS (resembling iret stack frame)
            "push rcx;",                  // Push userland return pointer
            "push 0;",                    // Fake error code

            "cld;",                       // Clear direction flag, required by ABI when running any Rust code in the kernel.

            push_scratch!(),
            push_preserved!(),

            // Call inner funtion
            "call {syscall_handler};",

            pop_preserved!(),
            pop_scratch!(),

            "swapgs;",                  // Restore user GSBASE by swapping GSBASE and KGSBASE.
            "add rsp,8;",               // Error code
            "pop rcx;",                 // Pop userland return pointer
            "add rsp, 8;",              // Pop fake userspace CS
            "pop r11;",                 // Pop rflags
            "pop rsp;",                 // Restore userland stack pointer
            "sysretq;",                 // Return into userland; RCX=>RIP,R11=>RFLAGS
        ), 

        syscall_handler = sym syscall_handler,
        usp = const(offset_of!(ProcessorControlRegion, userland_stack_ptr_tmp)),
        ksp = const(offset_of!(ProcessorControlRegion, kernal_stack_ptr)),
        ss_sel = const(USER_DATA_SELECTOR_INDEX),
        cs_sel = const(USER_CODE_SELECTOR_INDEX),

        options(noreturn));
    }
}

unsafe extern "C" fn syscall_handler() {
    userland_timer_end();

    let stack = InterruptStack::current();

    let n = stack.scratch.rax;
    let context = SyscallArgs::from_stack(stack);

    execute_syscall(n, context);

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
