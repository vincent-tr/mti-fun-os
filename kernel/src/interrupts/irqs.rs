use log::error;

use crate::{devices, interrupts::InterruptStack, user::thread};

pub const IRQ0: u8 = 32;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Irq {
    LocalApicTimer = IRQ0,
    LocalApicError,
}

pub fn lapic_timer_interrupt_handler(_stack: &mut InterruptStack) {
    let _userland_timer = thread::UserlandTimerInterruptScope::new();

    thread::thread_next();

    devices::local_apic::end_of_interrupt();
}

pub fn lapic_error_interrupt_handler(_stack: &mut InterruptStack) {
    error!(
        "Local APIC internal error: {:?}",
        devices::local_apic::current_errors()
    );

    devices::local_apic::end_of_interrupt();
}
