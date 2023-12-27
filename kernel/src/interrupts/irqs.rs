use log::error;

use crate::{
    devices,
    interrupts::InterruptStack,
    user::thread::{self, userland_timer_begin, userland_timer_end},
};

pub const IRQ0: u8 = 32;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Irq {
    LocalApicTimer = IRQ0,
    LocalApicError,
}

pub fn lapic_timer_interrupt_handler(_stack: &mut InterruptStack) {
    userland_timer_end();

    thread::thread_next();

    devices::local_apic::end_of_interrupt();

    userland_timer_begin();
}

pub fn lapic_error_interrupt_handler(_stack: &mut InterruptStack) {
    error!(
        "Local APIC internal error: {:?}",
        devices::local_apic::current_errors()
    );

    devices::local_apic::end_of_interrupt();
}
