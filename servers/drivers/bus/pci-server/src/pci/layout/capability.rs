/// Capability structure as defined in the PCI specification.
///
/// Each capability is a linked list of structures, where each structure has an ID and a pointer to the next structure.
/// The ID indicates the type of capability, and the next pointer allows for chaining multiple capabilities together.
#[derive(Default, Debug, Clone, Copy)]
#[repr(C, align(4))]
pub struct Capability {
    /// The ID of the capability, which indicates the type of capability (e.g., Power Management, MSI, etc.).
    pub id: u8,

    /// The offset to the next capability in the linked list. If this is 0, it indicates the end of the list.
    pub next: u8,

    /// The first 2 bytes of the capability data, the rest can be read using the offset from the configuration space
    pub data: [u8; 2],
}
