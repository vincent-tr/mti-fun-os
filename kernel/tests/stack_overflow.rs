#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

mod common;

use common::{exit_qemu, QemuExitCode};
use kernel::{logging, print};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    logging::init();

    print!("stack_overflow::stack_overflow...\t");

    kernel::gdt::init();
    init_test_idt();

    panic!("Execution continued after stack overflow");
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow(); // for each recursion, the return address is pushed
    volatile::Volatile::new(0).read(); // prevent tail recursion optimizations
}

fn init_test_idt() {
    static mut TEST_IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

    unsafe {
        TEST_IDT
            .double_fault
            .set_handler_fn(test_double_fault_handler)
            .set_stack_index(kernel::gdt::DOUBLE_FAULT_IST_INDEX);
    }

    TEST_IDT.load();
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}
