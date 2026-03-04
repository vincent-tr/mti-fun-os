use core::{fmt, mem};

use bit_field::BitField;

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, align(4))]
pub struct CommonHeader {
    pub vendor_id: u16,           // 0x00
    pub device_id: u16,           // 0x02
    pub command: CommandRegister, // 0x04
    pub status: StatusRegister,   // 0x06
    pub revision_id: u8,          // 0x08
    pub prog_if: u8,              // 0x09
    pub subclass: u8,             // 0x0A
    pub class_code: u8,           // 0x0B
    pub cache_line_size: u8,      // 0x0C
    pub latency_timer: u8,        // 0x0D
    pub header_type: HeaderType,  // 0x0E
    pub bist: u8,                 // 0x0F
}

const _: () = assert!(mem::size_of::<CommonHeader>() == 16);

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, align(4))]
pub struct GeneralDeviceHeader {
    pub common: CommonHeader,     // 0x00-0x0F
    pub bar0: Bar,                // 0x10
    pub bar1: Bar,                // 0x14
    pub bar2: Bar,                // 0x18
    pub bar3: Bar,                // 0x1C
    pub bar4: Bar,                // 0x20
    pub bar5: Bar,                // 0x24
    pub cardbus_cis_pointer: u32, // 0x28
    pub subsystem_vendor_id: u16, // 0x2C
    pub subsystem_id: u16,        // 0x2E
    pub expansion_rom_base: u32,  // 0x30
    pub capabilities_ptr: u8,     // 0x34
    pub reserved1: [u8; 7],       // 0x35-0x3B
    pub interrupt_line: u8,       // 0x3C
    pub interrupt_pin: u8,        // 0x3D
    pub min_grant: u8,            // 0x3E
    pub max_latency: u8,          // 0x3F
}

const _: () = assert!(mem::size_of::<GeneralDeviceHeader>() == 64);

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, align(4))]
pub struct PciToPciBridgeHeader {
    pub common: CommonHeader,      // 0x00-0x0F
    pub bar0: Bar,                 // 0x10
    pub bar1: Bar,                 // 0x14
    pub primary_bus: u8,           // 0x18
    pub secondary_bus: u8,         // 0x19
    pub subordinate_bus: u8,       // 0x1A
    pub secondary_latency: u8,     // 0x1B
    pub io_base: u8,               // 0x1C
    pub io_limit: u8,              // 0x1D
    pub secondary_status: u16,     // 0x1E
    pub memory_base: u16,          // 0x20
    pub memory_limit: u16,         // 0x22
    pub prefetch_base: u16,        // 0x24
    pub prefetch_limit: u16,       // 0x26
    pub prefetch_base_upper: u32,  // 0x28
    pub prefetch_limit_upper: u32, // 0x2C
    pub io_base_upper: u16,        // 0x30
    pub io_limit_upper: u16,       // 0x32
    pub capabilities_ptr: u8,      // 0x34
    pub reserved1: [u8; 3],        // 0x35-0x37
    pub expansion_rom_base: u32,   // 0x38
    pub interrupt_line: u8,        // 0x3C
    pub interrupt_pin: u8,         // 0x3D
    pub bridge_control: u16,       // 0x3E
}

const _: () = assert!(mem::size_of::<PciToPciBridgeHeader>() == 64);

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, align(4))]
pub struct PciToCardBusBridgeHeader {
    pub common: CommonHeader,       // 0x00-0x0F
    pub socket_explorer_base: u32,  // 0x10
    pub capabilities_ptr: u8,       // 0x14
    pub reserved1: [u8; 3],         // 0x15-0x17
    pub secondary_status: u16,      // 0x18
    pub bus_number: u8,             // 0x1A
    pub cardbus_number: u8,         // 0x1B
    pub subordinate_bus_number: u8, // 0x1C
    pub cardbus_latency_timer: u8,  // 0x1D
    pub memory_base_0: u32,         // 0x20
    pub memory_limit_0: u32,        // 0x24
    pub memory_base_1: u32,         // 0x28
    pub memory_limit_1: u32,        // 0x2C
    pub io_base_0: u32,             // 0x30
    pub io_limit_0: u32,            // 0x34
    pub io_base_1: u32,             // 0x38
    pub io_limit_1: u32,            // 0x3C
    pub interrupt_line: u8,         // 0x40
    pub interrupt_pin: u8,          // 0x41
    pub bridge_control: u16,        // 0x42
    pub subsystem_vendor_id: u16,   // 0x44
    pub subsystem_id: u16,          // 0x46
    pub legacy_mode_base: u32,      // 0x48
}

#[derive(Clone, Copy)]
#[repr(C)]
pub union Header {
    pub common: CommonHeader,
    pub general_device: GeneralDeviceHeader,
    pub pci_bridge: PciToPciBridgeHeader,
    pub cardbus_bridge: PciToCardBusBridgeHeader,
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // SAFETY: We can read the common header to determine the header type, and then read the appropriate variant based on that.
        let common = unsafe { &self.common };

        match common.header_type.r#type() {
            0x00 => f
                .debug_tuple("Header::GeneralDevice")
                .field(unsafe { &self.general_device })
                .finish(),
            0x01 => f
                .debug_tuple("Header::PciBridge")
                .field(unsafe { &self.pci_bridge })
                .finish(),
            0x02 => f
                .debug_tuple("Header::CardBusBridge")
                .field(unsafe { &self.cardbus_bridge })
                .finish(),
            _ => f
                .debug_tuple("Header::Unknown")
                .field(&format_args!(
                    "header_type={:#04x}",
                    common.header_type.r#type()
                ))
                .finish(),
        }
    }
}

#[allow(dead_code)]
impl Header {
    pub fn common(&self) -> &CommonHeader {
        // SAFETY: All variants of the header start with the common header, so we can safely read it regardless of the actual header type.
        unsafe { &self.common }
    }

    pub fn general_device(&self) -> Option<&GeneralDeviceHeader> {
        if self.common().header_type.r#type() == 0x00 {
            // SAFETY: We have checked that the header type is for a general device, so it is safe to read the general device header.
            Some(unsafe { &self.general_device })
        } else {
            None
        }
    }

    pub fn pci_bridge(&self) -> Option<&PciToPciBridgeHeader> {
        if self.common().header_type.r#type() == 0x01 {
            // SAFETY: We have checked that the header type is for a PCI-to-PCI bridge, so it is safe to read the PCI bridge header.
            Some(unsafe { &self.pci_bridge })
        } else {
            None
        }
    }

    pub fn cardbus_bridge(&self) -> Option<&PciToCardBusBridgeHeader> {
        if self.common().header_type.r#type() == 0x02 {
            // SAFETY: We have checked that the header type is for a PCI-to-CardBus bridge, so it is safe to read the CardBus bridge header.
            Some(unsafe { &self.cardbus_bridge })
        } else {
            None
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct CommandRegister(u16);

#[allow(dead_code)]
impl CommandRegister {
    pub fn memory_space_enabled(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn enable_memory_space(&mut self, enabled: bool) {
        self.0.set_bit(1, enabled);
    }

    pub fn io_space_enabled(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn enable_io_space(&mut self, enabled: bool) {
        self.0.set_bit(0, enabled);
    }

    pub fn bus_master_enabled(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn enable_bus_master(&mut self, enabled: bool) {
        self.0.set_bit(2, enabled);
    }

    pub fn interrupt_disabled(&self) -> bool {
        self.0.get_bit(10)
    }

    pub fn disable_interrupt(&mut self, disabled: bool) {
        self.0.set_bit(10, disabled);
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct StatusRegister(u16);

#[allow(dead_code)]
impl StatusRegister {
    pub fn interrupt_status(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn capabilities_list(&self) -> bool {
        self.0.get_bit(4)
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct HeaderType(u8);

impl HeaderType {
    pub fn r#type(&self) -> u8 {
        self.0.get_bits(0..7)
    }

    pub fn multi_function(&self) -> bool {
        self.0.get_bit(7)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub union Bar {
    pub raw: u32,
    pub io_space: IoSpaceBar,
    pub memory_space: MemorySpaceBar,
}

impl Default for Bar {
    fn default() -> Self {
        Self { raw: 0 }
    }
}

impl fmt::Debug for Bar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_io() {
            f.debug_tuple("Bar::IoSpace")
                .field(&unsafe { self.io_space })
                .finish()
        } else {
            f.debug_tuple("Bar::MemorySpace")
                .field(&unsafe { self.memory_space })
                .finish()
        }
    }
}

impl From<u32> for Bar {
    fn from(raw: u32) -> Self {
        Self { raw }
    }
}

impl From<Bar> for u32 {
    fn from(bar: Bar) -> Self {
        unsafe { bar.raw }
    }
}

impl Bar {
    pub fn is_implemented(&self) -> bool {
        (unsafe { self.raw }) != 0
    }

    pub fn is_memory(&self) -> bool {
        !(unsafe { self.raw }).get_bit(0)
    }

    pub fn is_io(&self) -> bool {
        (unsafe { self.raw }).get_bit(0)
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct MemorySpaceBar(u32);

impl MemorySpaceBar {
    pub fn r#type(&self) -> u8 {
        self.0.get_bits(1..3) as u8
    }

    pub fn is_32_bit(&self) -> bool {
        self.r#type() == 0x00
    }

    pub fn is_64_bit(&self) -> bool {
        self.r#type() == 0x02
    }

    pub fn prefetchable(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn address(&self) -> u32 {
        self.0.get_bits(4..32) << 4
    }

    pub fn set_address(&mut self, address: u32) {
        self.0.set_bits(4..32, address >> 4);
    }

    pub fn set_hightest_address(&mut self) {
        // The the highest address possible (useful to get BAR size)
        self.set_address(0xFFFF_FFFF);
    }

    pub fn read_size(&self) -> u32 {
        // The size of the memory space is encoded in the bits that are set to 0 when we write all 1s to the BAR
        (!self.0.get_bits(4..32)) << 4
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct IoSpaceBar(u32);

impl IoSpaceBar {
    pub fn address(&self) -> u32 {
        self.0.get_bits(2..32) << 2
    }

    pub fn set_address(&mut self, address: u32) {
        self.0.set_bits(2..32, address >> 2);
    }

    pub fn set_hightest_address(&mut self) {
        // The the highest address possible (useful to get BAR size)
        self.set_address(0xFFFF_FFFF);
    }

    pub fn read_size(&self) -> u32 {
        // The size of the I/O space is encoded in the bits that are set to 0 when we write all 1s to the BAR
        (!self.0.get_bits(2..32)) << 2
    }
}

#[derive(Default, Clone, Copy)]
#[repr(C)]
pub struct MemorySpaceBar64 {
    low: MemorySpaceBar,
    high: u32,
}

impl From<(u32, u32)> for MemorySpaceBar64 {
    fn from((low, high): (u32, u32)) -> Self {
        Self {
            low: MemorySpaceBar(low),
            high,
        }
    }
}

impl From<MemorySpaceBar64> for (u32, u32) {
    fn from(bar: MemorySpaceBar64) -> Self {
        (bar.low.0, bar.high)
    }
}

impl MemorySpaceBar64 {
    pub unsafe fn from_bars(low: Bar, high: Bar) -> Self {
        unsafe {
            Self {
                low: low.memory_space,
                high: high.raw,
            }
        }
    }

    pub fn prefetchable(&self) -> bool {
        self.low.prefetchable()
    }

    pub fn address(&self) -> u64 {
        ((self.high as u64) << 32) | (self.low.address() as u64)
    }

    pub fn set_address(&mut self, address: u64) {
        self.low.set_address(address as u32);
        self.high = (address >> 32) as u32;
    }

    pub fn set_hightest_address(&mut self) {
        // The the highest address possible (useful to get BAR size)
        self.set_address(0xFFFF_FFFF_FFFF_FFFF);
    }

    pub fn read_size(&self) -> u64 {
        // The size of the memory space is encoded in the bits that are set to 0 when we write all 1s to the BAR
        (!self.low.0.get_bits(4..32) as u64) << 4 | (!self.high as u64) << 32
    }
}

impl fmt::Debug for MemorySpaceBar64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MemorySpaceBar64")
            .field(&format_args!("{:#018x}", self.address()))
            .finish()
    }
}
