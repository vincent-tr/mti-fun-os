use alloc::sync::Arc;

use crate::user::ipc::PortSender;

/// IRQ (Interrupt Request) handling for user space.
#[derive(Debug)]
pub struct Irq {
    irq: u8,
    port: Arc<PortSender>,
}

/// Called by the ISR management code when a device IRQ is triggered.
pub fn handle_irq(irq: u8) {
    let _ = irq;
}
