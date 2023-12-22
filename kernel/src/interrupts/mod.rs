#[macro_use]
mod handler;
mod exceptions;
mod syscalls;
mod irqs;

use core::arch::asm;

use crate::gdt::{self, user_data_selector};
use lazy_static::lazy_static;
use x86_64::{structures::idt::InterruptDescriptorTable, registers::rflags::RFlags};
use crate::memory::VirtAddr;

use self::handler::init_process_control_region;

pub const USERLAND_RFLAGS: RFlags = RFlags::INTERRUPT_FLAG;
pub use self::handler::InterruptStack;
pub use self::irqs::Irq;

// Note:
// If kernel is entered with INT exception or irq, it should return to userland with IRET.
// If kernel is entered with SYSCALL, it should return to userland with SYSRET.

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(exceptions::double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        // TODO: setup proper kernel stack
        // TODO: fill all exceptions
        idt.page_fault.set_handler_fn(exceptions::page_fault_handler);
        idt.general_protection_fault.set_handler_fn(exceptions::general_protection_fault_handler);
        idt.invalid_opcode.set_handler_fn(exceptions::invalid_opcode_handler);

        idt[Irq::LocalApicTimer as usize].set_handler_fn(irqs::lapic_timer_interrupt_handler);
        idt[Irq::LocalApicError as usize].set_handler_fn(irqs::lapic_error_interrupt_handler);

        idt
    };
}

pub fn init_base() {
    IDT.load();
}

pub fn init_userland() {
    init_process_control_region();

    syscalls::init();
}

pub fn switch_to_userland(user_code: VirtAddr, user_stack_top: VirtAddr) -> ! {
    // Initial switch to userland using sysretq

    unsafe {
        asm!(concat!(
            "mov ds, {user_data_seg:x};",     // Set userland data segment
            "mov es, {user_data_seg:x};",     // Set userland data segment
            "mov fs, {user_data_seg:x};",     // Set userland data segment
            "mov gs, {user_data_seg:x};",     // Set userland data segment
            "mov rsp, {user_stack};",         // Set userland stack pointer
            "sysretq;",                       // Return into userland; RCX=>RIP,R11=>RFLAGS
        ),

        user_data_seg = in(reg) user_data_selector().0,
        user_stack = in(reg) user_stack_top.as_u64(),
        in("rcx") user_code.as_u64(), // Set userland return pointer
        in("r11")USERLAND_RFLAGS.bits(), // Set rflags

        options(noreturn));
    }
}