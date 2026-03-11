use core::{marker::PhantomData, mem};

use crate::kobject;

#[derive(Debug)]
pub struct IoPortIndirectRegion<Word> {
    address_port: kobject::PortRange,
    data_port: kobject::PortRange,
    _word: PhantomData<Word>,
}

impl<Word> IoPortIndirectRegion<Word> {
    /// Opens an indirect I/O port region. The `address_port` is used to select the register, and the `data_port` is used to read/write the value of the selected register.
    pub fn open(address_port: u16, data_port: u16) -> Result<Self, kobject::Error> {
        let address_port = kobject::PortRange::open(address_port, 1, kobject::PortAccess::WRITE)?;
        let data_port = kobject::PortRange::open(
            data_port,
            1,
            kobject::PortAccess::READ | kobject::PortAccess::WRITE,
        )?;

        Ok(Self {
            address_port,
            data_port,
            _word: PhantomData,
        })
    }
}

impl IoPortIndirectRegion<u8> {
    /// Read a byte from the indirect I/O port region at the given offset.
    pub fn read(&self, offset: usize) -> u8 {
        assert!(
            offset < u8::MAX as usize,
            "Offset must be less than 256 for byte access"
        );
        self.address_port
            .write8(0, offset as u8)
            .expect("Failed to write to address port");
        self.data_port
            .read8(0)
            .expect("Failed to read from data port")
    }

    /// Write a byte to the indirect I/O port region at the given offset.
    pub fn write(&self, offset: usize, value: u8) {
        assert!(
            offset < u8::MAX as usize,
            "Offset must be less than 256 for byte access"
        );
        self.address_port
            .write8(0, offset as u8)
            .expect("Failed to write to address port");
        self.data_port
            .write8(0, value)
            .expect("Failed to write to data port");
    }
}

impl IoPortIndirectRegion<u16> {
    /// Read a word from the indirect I/O port region at the given offset.
    pub fn read(&self, offset: usize) -> u16 {
        assert!(
            offset < u16::MAX as usize,
            "Offset must be less than 65536 for word access"
        );
        assert!(
            offset % mem::size_of::<u16>() == 0,
            "Offset must be aligned to 2 bytes for word access"
        );
        self.address_port
            .write16(0, offset as u16)
            .expect("Failed to write to address port");
        self.data_port
            .read16(0)
            .expect("Failed to read from data port")
    }

    /// Write a word to the indirect I/O port region at the given offset.
    pub fn write(&self, offset: usize, value: u16) {
        assert!(
            offset < u16::MAX as usize,
            "Offset must be less than 65536 for word access"
        );
        assert!(
            offset % mem::size_of::<u16>() == 0,
            "Offset must be aligned to 2 bytes for word access"
        );
        self.address_port
            .write16(0, offset as u16)
            .expect("Failed to write to address port");
        self.data_port
            .write16(0, value)
            .expect("Failed to write to data port");
    }
}

impl IoPortIndirectRegion<u32> {
    /// Read a double word from the indirect I/O port region at the given offset.
    pub fn read(&self, offset: usize) -> u32 {
        assert!(
            offset < u32::MAX as usize,
            "Offset must be less than 4294967296 for dword access"
        );
        assert!(
            offset % mem::size_of::<u32>() == 0,
            "Offset must be aligned to 4 bytes for dword access"
        );
        self.address_port
            .write32(0, offset as u32)
            .expect("Failed to write to address port");
        self.data_port
            .read32(0)
            .expect("Failed to read from data port")
    }

    /// Write a double word to the indirect I/O port region at the given offset.
    pub fn write(&self, offset: usize, value: u32) {
        assert!(
            offset < u32::MAX as usize,
            "Offset must be less than 4294967296 for dword access"
        );
        assert!(
            offset % mem::size_of::<u32>() == 0,
            "Offset must be aligned to 4 bytes for dword access"
        );
        self.address_port
            .write32(0, offset as u32)
            .expect("Failed to write to address port");
        self.data_port
            .write32(0, value)
            .expect("Failed to write to data port");
    }
}
