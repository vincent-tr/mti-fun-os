#[macro_use]
mod handler;
mod exceptions;
mod irqs;
mod syscalls;

use core::arch::asm;

use crate::gdt::{self, user_data_selector};
use crate::memory::VirtAddr;
use lazy_static::lazy_static;
use x86_64::{registers::rflags::RFlags, structures::idt::InterruptDescriptorTable};

use self::handler::init_process_control_region;

pub const USERLAND_RFLAGS: RFlags = RFlags::INTERRUPT_FLAG;
pub use self::handler::InterruptStack;
pub use self::irqs::Irq;
pub use self::syscalls::SyscallArgs;

// Note:
// If kernel is entered with INT exception or irq, it should return to userland with IRET.
// If kernel is entered with SYSCALL, it should return to userland with SYSRET.

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {

        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_addr(native_error_handler!(exceptions::double_fault_handler))
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            // TODO: fill all exceptions
            idt.page_fault
                .set_handler_addr(native_error_handler!(exceptions::page_fault_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);
            idt.general_protection_fault
                .set_handler_addr(native_error_handler!(exceptions::general_protection_fault_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);
            idt.invalid_opcode
            .set_handler_addr(native_handler!(exceptions::invalid_opcode_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt[Irq::LocalApicTimer as usize]
                .set_handler_addr(native_handler!(irqs::lapic_timer_interrupt_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);
            idt[Irq::LocalApicError as usize]
                .set_handler_addr(native_handler!(irqs::lapic_error_interrupt_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);
        }

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

pub fn syscall_switch(syscall_number: usize) -> ! {
    // Initial context switch to interrupt stack, using syscall
    unsafe {
        asm!(concat!(
            "mov ds, {user_data_seg:x};",     // Set userland data segment
            "mov es, {user_data_seg:x};",     // Set userland data segment
            "mov fs, {user_data_seg:x};",     // Set userland data segment
            "mov gs, {user_data_seg:x};",     // Set userland data segment
            "syscall;",                       // Run syscall (will never return)
        ),

        user_data_seg = in(reg) user_data_selector().0,
        in("rax") syscall_number,

        options(noreturn));
    }
}
