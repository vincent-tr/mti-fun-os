#[macro_use]
mod handler;
mod exceptions;
mod irqs;
mod syscalls;

use core::arch::asm;
use core::intrinsics::unreachable;

use crate::gdt;
use crate::memory::VirtAddr;
use lazy_static::lazy_static;
use x86_64::registers::model_specific::FsBase;
use x86_64::{registers::rflags::RFlags, structures::idt::InterruptDescriptorTable};

use self::handler::init_process_control_region;

pub const USERLAND_RFLAGS: RFlags = RFlags::INTERRUPT_FLAG;
pub use self::exceptions::Exception;
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
            // Fatal
            idt.double_fault
                .set_handler_addr(native_error_handler!(exceptions::double_fault_handler))
                .set_stack_index(gdt::FATAL_FAULT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.machine_check
                .set_handler_addr(native_handler!(exceptions::machine_check_handler))
                .set_stack_index(gdt::FATAL_FAULT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            // Others
            idt.divide_error
                .set_handler_addr(native_handler!(exceptions::divide_error_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.debug
                .set_handler_addr(native_handler!(exceptions::debug_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.non_maskable_interrupt
                .set_handler_addr(native_handler!(exceptions::non_maskable_interrupt_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.breakpoint
                .set_handler_addr(native_handler!(exceptions::breakpoint_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.overflow
                .set_handler_addr(native_handler!(exceptions::overflow_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.bound_range_exceeded
                .set_handler_addr(native_handler!(exceptions::bound_range_exceeded_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.invalid_opcode
                .set_handler_addr(native_handler!(exceptions::invalid_opcode_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.device_not_available
                .set_handler_addr(native_handler!(exceptions::device_not_available_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.invalid_tss
                .set_handler_addr(native_error_handler!(exceptions::invalid_tss_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.segment_not_present
                .set_handler_addr(native_error_handler!(exceptions::segment_not_present_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.stack_segment_fault
                .set_handler_addr(native_error_handler!(exceptions::stack_segment_fault_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.general_protection_fault
                .set_handler_addr(native_error_handler!(exceptions::general_protection_fault_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.page_fault
                .set_handler_addr(native_error_handler!(exceptions::page_fault_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.x87_floating_point
                .set_handler_addr(native_handler!(exceptions::x87_floating_point_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.alignment_check
                .set_handler_addr(native_error_handler!(exceptions::alignment_check_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.simd_floating_point
                .set_handler_addr(native_handler!(exceptions::simd_floating_point_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.virtualization
                .set_handler_addr(native_handler!(exceptions::virtualization_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.cp_protection_exception
                .set_handler_addr(native_error_handler!(exceptions::cp_protection_exception_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.hv_injection_exception
                .set_handler_addr(native_handler!(exceptions::hv_injection_exception_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.vmm_communication_exception
                .set_handler_addr(native_error_handler!(exceptions::vmm_communication_exception_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt.security_exception
                .set_handler_addr(native_error_handler!(exceptions::security_exception_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);


            idt[Irq::LocalApicTimer as u8]
                .set_handler_addr(native_handler!(irqs::lapic_timer_interrupt_handler))
                .set_stack_index(gdt::INTERRUPT_IST_INDEX)
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

            idt[Irq::LocalApicError as u8]
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

pub fn tls_reg_read() -> VirtAddr {
    FsBase::read()
}

pub fn tls_reg_write(addr: VirtAddr) {
    FsBase::write(addr);
}

pub fn syscall_switch(
    syscall_number: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> ! {
    // Initial context switch to interrupt stack, using syscall
    unsafe {
        asm!(concat!(
            "mov ds, {user_data_seg:x};",     // Set userland data segment
            "mov es, {user_data_seg:x};",     // Set userland data segment
            "mov fs, {user_data_seg:x};",     // Set userland data segment
            "mov gs, {user_data_seg:x};",     // Set userland data segment
            "syscall;",                       // Run syscall (will never return)
        ),

        user_data_seg = in(reg) gdt::USER_DATA_SELECTOR.0,
        in("rax") syscall_number,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        in("r9") arg6,
        options(noreturn));

        unreachable();
    }
}
