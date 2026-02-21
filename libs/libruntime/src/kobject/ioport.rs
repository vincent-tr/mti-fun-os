use libsyscalls::{HandleType, ioport};

use super::*;

/// I/O port range
#[derive(Debug)]
pub struct PortRange {
    handle: Handle,
}

impl KObject for PortRange {
    unsafe fn handle(&self) -> &Handle {
        &self.handle
    }

    fn into_handle(self) -> Handle {
        self.handle
    }

    unsafe fn from_handle_unchecked(handle: Handle) -> Self {
        Self { handle }
    }

    fn from_handle(handle: Handle) -> Result<Self, Error> {
        if !handle.valid() {
            return Err(Error::InvalidArgument);
        }
        if handle.r#type() != HandleType::PortRange {
            return Err(Error::InvalidArgument);
        }

        Ok(unsafe { Self::from_handle_unchecked(handle) })
    }
}

impl Clone for PortRange {
    fn clone(&self) -> Self {
        let handle = self.handle.clone();

        Self { handle }
    }
}

impl PortRange {
    /// Opens a new I/O port range with the specified access rights.
    pub fn open(from: u16, count: usize, access: PortAccess) -> Result<Self, Error> {
        let handle = ioport::open(from, count, access)?;

        Ok(Self { handle })
    }

    /// Reads an 8-bit value from the specified port index.
    pub fn read8(&self, index: u16) -> Result<u8, Error> {
        let value = ioport::read(&self.handle, index, 1)?;
        Ok(value as u8)
    }

    /// Reads a 16-bit value from the specified port index.
    pub fn read16(&self, index: u16) -> Result<u16, Error> {
        let value = ioport::read(&self.handle, index, 2)?;
        Ok(value as u16)
    }

    /// Reads a 32-bit value from the specified port index.
    pub fn read32(&self, index: u16) -> Result<u32, Error> {
        let value = ioport::read(&self.handle, index, 4)?;
        Ok(value as u32)
    }

    /// Writes an 8-bit value to the specified port index.
    pub fn write8(&self, index: u16, value: u8) -> Result<(), Error> {
        ioport::write(&self.handle, index, 1, value as usize)?;
        Ok(())
    }

    /// Writes a 16-bit value to the specified port index.
    pub fn write16(&self, index: u16, value: u16) -> Result<(), Error> {
        ioport::write(&self.handle, index, 2, value as usize)?;
        Ok(())
    }

    /// Writes a 32-bit value to the specified port index.
    pub fn write32(&self, index: u16, value: u32) -> Result<(), Error> {
        ioport::write(&self.handle, index, 4, value as usize)?;
        Ok(())
    }
}
