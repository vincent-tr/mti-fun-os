use bit_field::BitField;
use core::fmt;

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
        ((!self.0.get_bits(4..32)) + 1) << 4
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
        ((!self.0.get_bits(2..32)) + 1) << 2
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
        // Combine as 60-bit value: high bits shifted by 28 (32-4), low bits from get_bits (already at 0-27)
        let bar_value = ((self.high as u64) << (32 - 4)) | (self.low.0.get_bits(4..32) as u64);
        // Invert, add 1, shift back by 4 (same algorithm as 32-bit version)
        ((!bar_value) + 1) << 4
    }
}

impl fmt::Debug for MemorySpaceBar64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MemorySpaceBar64")
            .field(&format_args!("{:#018x}", self.address()))
            .finish()
    }
}
