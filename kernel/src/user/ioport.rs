use core::ops::Range;

use bitflags::bitflags;
use syscalls::Error;
use x86_64::instructions::port::Port;

bitflags! {
    /// Possible port access
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct PortAccess: u64 {
        /// No access
        const NONE = 0;

        /// Page can be read
        const READ = 1 << 0;

        /// Page can be written
        const WRITE = 1 << 1;

        /// Page can be executed
        const EXECUTE = 1 << 2;
    }
}

/// Represents a range of I/O ports with specific access rights.
#[derive(Debug)]
pub struct PortRange {
    range: Range<u16>,
    access: PortAccess,
}

impl PortRange {
    pub fn new(start: u16, end: u16, access: PortAccess) -> Self {
        assert!(start < end, "Invalid port range");
        Self {
            range: start..end,
            access,
        }
    }

    /// Returns the range of ports.
    pub fn range(&self) -> &Range<u16> {
        &self.range
    }

    /// Returns the number of ports in this range.
    pub fn len(&self) -> u16 {
        self.range.end - self.range.start
    }

    /// Returns the access rights for this port range.
    pub fn access(&self) -> PortAccess {
        self.access
    }

    /// Reads a value from the specified port index with the given word size.
    pub fn read(&self, index: u16, word_size: u8) -> Result<usize, Error> {
        if index >= self.len() {
            return Err(Error::InvalidArgument);
        }

        match word_size {
            1 | 2 | 4 => {}
            _ => return Err(Error::InvalidArgument),
        }

        if !self.access.contains(PortAccess::READ) {
            return Err(Error::MemoryAccessDenied);
        }

        let port_number = self.range.start + index;

        let value = match word_size {
            1 => (unsafe { Port::<u8>::new(port_number).read() }) as usize,
            2 => (unsafe { Port::<u16>::new(port_number).read() }) as usize,
            4 => (unsafe { Port::<u32>::new(port_number).read() }) as usize,
            _ => unreachable!(), // checked above
        };

        Ok(value)
    }

    /// Writes a value to the specified port index with the given word size.
    pub fn write(&self, index: u16, word_size: u8, value: usize) -> Result<(), Error> {
        if index >= self.len() {
            return Err(Error::InvalidArgument);
        }

        match word_size {
            1 => {
                if value > u8::MAX as usize {
                    return Err(Error::InvalidArgument);
                }
            }
            2 => {
                if value > u16::MAX as usize {
                    return Err(Error::InvalidArgument);
                }
            }
            4 => {
                if value > u32::MAX as usize {
                    return Err(Error::InvalidArgument);
                }
            }
            _ => return Err(Error::InvalidArgument),
        }

        if !self.access.contains(PortAccess::WRITE) {
            return Err(Error::MemoryAccessDenied);
        }

        let port_number = self.range.start + index;

        match word_size {
            1 => unsafe { Port::<u8>::new(port_number).write(value as u8) },
            2 => unsafe { Port::<u16>::new(port_number).write(value as u16) },
            4 => unsafe { Port::<u32>::new(port_number).write(value as u32) },
            _ => unreachable!(), // checked above
        };

        Ok(())
    }
}
