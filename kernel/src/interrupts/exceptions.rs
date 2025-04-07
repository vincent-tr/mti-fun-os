use x86_64::structures::{gdt::SegmentSelector, idt::PageFaultErrorCode};

use crate::{gdt, user::thread::thread_error};

use super::InterruptStack;
pub use syscalls::Exception;

pub fn divide_error_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: DIVIDE ERROR\n{:#?}", stack);
    }

    thread_error(Exception::DivideError);
}

pub fn debug_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: DEBUG\n{:#?}", stack);
    }

    thread_error(Exception::Debug);
}

pub fn non_maskable_interrupt_handler(stack: &mut InterruptStack) {
    // An non maskable interrupt exception (NMI) occurs as a result of system logic
    // signaling a non-maskable interrupt to the processor.
    //
    // The processor recognizes an NMI at an instruction boundary.
    // The saved instruction pointer points to the instruction immediately following the
    // boundary where the NMI was recognized.
    panic!("EXCEPTION: NMI\n{:#?}", stack);
}

pub fn breakpoint_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: BREAKPOINT\n{:#?}", stack);
    }

    thread_error(Exception::Breakpoint);
}

pub fn overflow_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: OVERFLOW\n{:#?}", stack);
    }

    thread_error(Exception::Overflow);
}

pub fn bound_range_exceeded_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}", stack);
    }

    thread_error(Exception::BoundRangeExceeded);
}

pub fn invalid_opcode_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: INVALID OPCODE\n{:#?}", stack);
    }

    thread_error(Exception::InvalidOpcode);
}

pub fn device_not_available_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack);
    }

    thread_error(Exception::DeviceNotAvailable);
}

pub fn double_fault_handler(stack: &mut InterruptStack) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack);
}

pub fn invalid_tss_handler(stack: &mut InterruptStack) {
    panic!("EXCEPTION: INVALID TSS\n{:#?}", stack);
}

pub fn segment_not_present_handler(stack: &mut InterruptStack) {
    panic!(
        "EXCEPTION: SEGMENT NOT PRESENT\n  Segment index: {}\n{:#?}",
        stack.error_code, stack
    );
}

pub fn stack_segment_fault_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!(
            "EXCEPTION: STACK SEGMENT FAULT\n  Segment index: {}\n{:#?}",
            stack.error_code, stack
        );
    }

    thread_error(Exception::StackSegmentFault(stack.error_code));
}

pub fn general_protection_fault_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!(
            "EXCEPTION: GENERAL PROTECTION FAULT\n  Segment index: {}\n{:#?}",
            stack.error_code, stack
        );
    }

    thread_error(Exception::GeneralProtectionFault(stack.error_code));
}

pub fn page_fault_handler(stack: &mut InterruptStack) {
    use x86_64::registers::control::Cr2;

    let accessed_address = Cr2::read().expect("Failed to read CR2 register");

    if !is_userland(stack) {
        let error_code = PageFaultErrorCode::from_bits_retain(stack.error_code as u64);
        let instruction_ptr = stack.iret.instruction_pointer;

        panic!(
            "EXCEPTION: PAGE FAULT\n  Error Code: {:?}\n  Accessed Address: {:#016x}\n  Instruction pointer: {:#016x}",
            error_code,
            accessed_address,
            instruction_ptr
        );
    }

    thread_error(Exception::PageFault(
        stack.error_code,
        accessed_address.as_u64() as usize,
    ));
}

pub fn x87_floating_point_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack);
    }

    thread_error(Exception::X87FloatingPoint);
}

pub fn alignment_check_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack);
    }

    thread_error(Exception::AlignmentCheck);
}

pub fn machine_check_handler(stack: &mut InterruptStack) -> ! {
    panic!("EXCEPTION: MACHINE CHECK\n{:#?}", stack);
}

pub fn simd_floating_point_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!("EXCEPTION: SIMD FLOATING POINT\n{:#?}", stack);
    }

    thread_error(Exception::SimdFloatingPoint);
}

pub fn virtualization_handler(stack: &mut InterruptStack) {
    panic!("EXCEPTION: VIRTUALIZATION (virtualization)\n{:#?}", stack);
}

pub fn cp_protection_exception_handler(stack: &mut InterruptStack) {
    if !is_userland(stack) {
        panic!(
            "EXCEPTION: CP PROTECTION\n  Error code: {}\n{:#?}",
            stack.error_code, stack
        );
    }

    thread_error(Exception::CpProtectionException(stack.error_code));
}

pub fn hv_injection_exception_handler(stack: &mut InterruptStack) {
    panic!("EXCEPTION: HV INJECTION (virtualization)\n{:#?}", stack);
}

pub fn vmm_communication_exception_handler(stack: &mut InterruptStack) {
    panic!(
        "EXCEPTION: VMM COMMUNICATION (virtualization)\n  Error code: {}\n{:#?}",
        stack.error_code, stack
    );
}

pub fn security_exception_handler(stack: &mut InterruptStack) {
    panic!(
        "EXCEPTION: SECURITY (virtualization)\n  Error code: {}\n{:#?}",
        stack.error_code, stack
    );
}

fn is_userland(stack: &mut InterruptStack) -> bool {
    SegmentSelector(stack.iret.code_segment.0) == gdt::USER_CODE_SELECTOR
}
