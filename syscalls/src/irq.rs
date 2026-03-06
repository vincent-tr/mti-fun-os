/// Process event
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IrqEvent {
    /// Vector number
    pub vector: u64,
}

/// Information about an IRQ.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IrqInfo {
    /// The address to send MSI messages to for this IRQ.
    pub msi_address: u64,

    /// The data to send in MSI messages for this IRQ.
    pub vector: u8,
}
