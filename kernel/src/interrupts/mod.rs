#[macro_use]
mod exceptions;

use crate::gdt;
use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(exceptions::double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        // TODO: setup proper kernel stack
        idt.page_fault.set_handler_fn(exceptions::page_fault_handler);

        idt
    };
}

pub fn init_idt() {
    IDT.load();
}
