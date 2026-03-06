/// Process event
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IrqEvent {
    /// IRQ number
    pub irq: u64,
}
