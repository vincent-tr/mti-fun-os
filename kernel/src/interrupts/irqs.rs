use log::info;
use x86_64::structures::idt::InterruptStackFrame;

use crate::devices;

pub const IRQ0: u8 = 32;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Irq {
    Timer = IRQ0,
}

// TODO: Properly push context
pub extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    info!(".");

    devices::local_apic::end_of_interrupt();
}
