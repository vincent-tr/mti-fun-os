use log::{info, error};
use x86_64::structures::idt::InterruptStackFrame;

use crate::devices;

pub const IRQ0: u8 = 32;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Irq {
    LocalApicTimer = IRQ0,
    LocalApicError,
}

// TODO: Properly push context
pub extern "x86-interrupt" fn lapic_timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    info!(".");

    devices::local_apic::end_of_interrupt();
}

// TODO: Properly push context
pub extern "x86-interrupt" fn lapic_error_interrupt_handler(stack_frame: InterruptStackFrame) {
    error!("Local APIC internal error: {:?}", devices::local_apic::current_errors());

    devices::local_apic::end_of_interrupt();
}
