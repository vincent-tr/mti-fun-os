use log::error;

use crate::{
    devices,
    interrupts::InterruptStack,
    user::{irq, thread, timer},
};

pub const IRQ0: u8 = 32;

pub const EXTERNAL_IRQ_START: u8 = IRQ0 + 2;
pub const EXTERNAL_IRQ_END: u8 = u8::MAX;

/// IRQ numbers for the system, including both local APIC and device IRQs.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Irq {
    LocalApicTimer = IRQ0,
    LocalApicError,
}

pub fn lapic_timer_interrupt_handler(_stack: &mut InterruptStack) {
    let _userland_timer = thread::UserlandTimerInterruptScope::new();

    thread::thread_next();

    // Note: when moving to multicore, make sure only one core calls this.
    timer::tick();

    devices::local_apic::end_of_interrupt();
}

pub fn lapic_error_interrupt_handler(_stack: &mut InterruptStack) {
    error!(
        "Local APIC internal error: {:?}",
        devices::local_apic::current_errors()
    );

    devices::local_apic::end_of_interrupt();
}

pub fn device_interrupt_handler(_stack: &mut InterruptStack, vector: u8) {
    irq::handle_irq(vector);

    devices::local_apic::end_of_interrupt();
}
