use core::fmt;

/// A network-ordered u16
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct NetU16([u8; 2]);

impl NetU16 {
    pub const ZERO: Self = Self::from_u16(0);

    pub const fn from_u16(value: u16) -> Self {
        Self(value.to_be_bytes())
    }

    pub const fn to_u16(&self) -> u16 {
        u16::from_be_bytes(self.0)
    }
}

impl From<u16> for NetU16 {
    fn from(value: u16) -> Self {
        Self::from_u16(value)
    }
}

impl From<NetU16> for u16 {
    fn from(value: NetU16) -> Self {
        value.to_u16()
    }
}

impl fmt::Debug for NetU16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_u16().fmt(f)
    }
}

impl fmt::Display for NetU16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_u16().fmt(f)
    }
}

impl fmt::LowerHex for NetU16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_u16().fmt(f)
    }
}

impl fmt::UpperHex for NetU16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_u16().fmt(f)
    }
}

/// A network-ordered u32
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct NetU32([u8; 4]);

impl NetU32 {
    pub const ZERO: Self = Self::from_u32(0);

    pub const fn from_u32(value: u32) -> Self {
        Self(value.to_be_bytes())
    }

    pub const fn to_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }
}

impl From<u32> for NetU32 {
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

impl From<NetU32> for u32 {
    fn from(value: NetU32) -> Self {
        value.to_u32()
    }
}

impl fmt::Debug for NetU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_u32().fmt(f)
    }
}

impl fmt::Display for NetU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_u32().fmt(f)
    }
}

impl fmt::LowerHex for NetU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_u32().fmt(f)
    }
}

impl fmt::UpperHex for NetU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_u32().fmt(f)
    }
}
