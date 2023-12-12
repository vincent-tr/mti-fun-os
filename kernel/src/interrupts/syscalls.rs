use super::handler::InterruptStack;
use core::arch::asm;
use log::info;
use memoffset::offset_of;

use crate::gdt::{USER_CODE_SELECTOR_INDEX, USER_DATA_SELECTOR_INDEX};
use crate::memory::VirtAddr;
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

            push_scratch!(),
            push_preserved!(),

            // Call inner funtion
            "call {syscall_handler};",

            pop_preserved!(),
            pop_scratch!(),

            "swapgs;",                  // Restore user GSBASE by swapping GSBASE and KGSBASE.
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
    let stack = InterruptStack::current();

    // TODO
    info!("SYSCALL {}", stack.scratch.rax);

    if stack.scratch.rax == 2 {
        panic!("end");
    }
}
