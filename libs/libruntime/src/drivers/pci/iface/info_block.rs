use alloc::vec::Vec;
use core::{mem, ptr, fmt, error};

use crate::drivers::pci::types::{PciAddress, PciClass, PciDeviceId};

/// Version of the InfoBlock format.
const VERSION: u32 = 1;

/// Device information.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PciDeviceInfo {
    pub address: PciAddress,
    pub device_id: PciDeviceId,
    pub class: PciClass,
}

/// PCI devices info block format for IPC communication.
pub struct InfoBlock;

impl InfoBlock {
    /// Builds a PCI devices list into the provided buffer.
    ///
    /// Returns `Ok(bytes_written)` on success, or `Err(required_size)` if the buffer is too small.
    pub fn build(devices: &[PciDeviceInfo], buffer: &mut [u8]) -> Result<usize, usize> {
        let required_size = Self::calculate_size(devices);
        if buffer.len() < required_size {
            return Err(required_size);
        }

        // Write header
        let header = Header {
            version: VERSION,
            entry_count: devices.len() as u32,
        };
        unsafe {
            ptr::write(buffer.as_mut_ptr() as *mut Header, header);
        }

        // Write all device info entries
        let entries_offset = mem::size_of::<Header>();
        let entries_ptr = unsafe { buffer.as_mut_ptr().add(entries_offset) as *mut PciDeviceInfo };

        for (i, device) in devices.iter().enumerate() {
            unsafe {
                ptr::write(entries_ptr.add(i), *device);
            }
        }

        Ok(required_size)
    }

    /// Reads a PCI devices list from the provided buffer.
    ///
    /// Returns a vector of PciDeviceInfo on success, or an error if we get the wrong version.
    pub fn read(buffer: &[u8]) -> Result<Vec<PciDeviceInfo>, InfoBlockReadError> {
        assert!(buffer.len() >= mem::size_of::<Header>());

        // Read header
        let header = unsafe { &*(buffer.as_ptr() as *const Header) };

        if header.version != VERSION {
            return Err(InfoBlockReadError::InvalidVersion);
        }

        let entry_count = header.entry_count as usize;
        let required_size = Self::calculate_size_for_count(entry_count);
        assert!(buffer.len() >= required_size);

        let mut result = Vec::with_capacity(entry_count);
        let entries_offset = mem::size_of::<Header>();
        let entries_ptr = unsafe { buffer.as_ptr().add(entries_offset) as *const PciDeviceInfo };

        for i in 0..entry_count {
            let device_info = unsafe { ptr::read(entries_ptr.add(i)) };
            result.push(device_info);
        }

        Ok(result)
    }

    /// Calculates the total size needed to store the given devices.
    fn calculate_size(devices: &[PciDeviceInfo]) -> usize {
        Self::calculate_size_for_count(devices.len())
    }

    /// Calculates the total size needed to store the given number of devices.
    fn calculate_size_for_count(count: usize) -> usize {
        mem::size_of::<Header>() + count * mem::size_of::<PciDeviceInfo>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoBlockReadError {
    InvalidVersion,
}

impl fmt::Display for InfoBlockReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersion => write!(f, "Invalid InfoBlock version"),
        }
    }
}

impl error::Error for InfoBlockReadError {}

#[derive(Debug)]
#[repr(C)]
struct Header {
    pub version: u32,
    pub entry_count: u32,
}
