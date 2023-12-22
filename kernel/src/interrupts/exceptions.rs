use x86_64::structures::idt::PageFaultErrorCode;

use super::InterruptStack;

pub fn page_fault_handler(stack: &mut InterruptStack) {
    use x86_64::registers::control::Cr2;

    let accessed_address = Cr2::read();
    let error_code = PageFaultErrorCode::from_bits_retain(stack.error_code as u64);
    let instruction_ptr = stack.iret.instruction_pointer;

    panic!(
        "EXCEPTION: PAGE FAULT\n  Error Code: {:?}\n  Accessed Address: {:#016x}\n  Instruction pointer: {:#016x}",
        error_code,
        accessed_address,
        instruction_ptr
    );
}

pub fn general_protection_fault_handler(stack: &mut InterruptStack) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\n  Segment index: {}\n{:#?}",
        stack.error_code, stack
    );
}

pub fn invalid_opcode_handler(stack: &mut InterruptStack) {
    panic!("EXCEPTION: INVALID OPCODE\n{:#?}", stack);
}

pub fn double_fault_handler(stack: &mut InterruptStack) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack);
}
