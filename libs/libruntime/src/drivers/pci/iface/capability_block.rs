use alloc::vec::Vec;
use core::{error, fmt, mem, ptr};

/// Version of the CapabilityBlock format.
const VERSION: u32 = 1;

/// Information about a PCI capability (e.g., power management, MSI, etc.).
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct CapabilityInfo {
    /// The index of the capability, used as an identifier to access the data.
    pub index: usize,

    /// The ID of the capability.
    pub id: u8,

    /// The maximum size of the capability data.
    pub max_size: usize,
}

impl CapabilityInfo {
    /// Power management capability ID
    pub const POWER_MANAGEMENT_CAPABILITY_ID: u8 = 0x01;
    
    /// MSI capability ID
    pub const MSI_CAPABILITY_ID: u8 = 0x05;
}

/// PCI capabilities info block format for IPC communication.
pub struct CapabilityBlock;

impl CapabilityBlock {
    /// Builds a PCI capabilities list into the provided buffer.
    ///
    /// Returns `Ok(bytes_written)` on success, or `Err(required_size)` if the buffer is too small.
    pub fn build(capabilities: &[CapabilityInfo], buffer: &mut [u8]) -> Result<usize, usize> {
        let required_size = Self::calculate_size(capabilities);
        if buffer.len() < required_size {
            return Err(required_size);
        }

        // Write header
        let header = Header {
            version: VERSION,
            entry_count: capabilities.len() as u32,
        };
        unsafe {
            ptr::write(buffer.as_mut_ptr() as *mut Header, header);
        }

        // Write all capability info entries
        let entries_offset = mem::size_of::<Header>();
        let entries_ptr = unsafe { buffer.as_mut_ptr().add(entries_offset) as *mut CapabilityInfo };

        for (i, capability) in capabilities.iter().enumerate() {
            unsafe {
                ptr::write(entries_ptr.add(i), *capability);
            }
        }

        Ok(required_size)
    }

    /// Reads a PCI capabilities list from the provided buffer.
    ///
    /// Returns a vector of CapabilityInfo on success, or an error if we get the wrong version.
    pub fn read(buffer: &[u8]) -> Result<Vec<CapabilityInfo>, CapabilityBlockReadError> {
        assert!(buffer.len() >= mem::size_of::<Header>());

        // Read header
        let header = unsafe { &*(buffer.as_ptr() as *const Header) };

        if header.version != VERSION {
            return Err(CapabilityBlockReadError::InvalidVersion);
        }

        let entry_count = header.entry_count as usize;
        let required_size = Self::calculate_size_for_count(entry_count);
        assert!(buffer.len() >= required_size);

        let mut result = Vec::with_capacity(entry_count);
        let entries_offset = mem::size_of::<Header>();
        let entries_ptr = unsafe { buffer.as_ptr().add(entries_offset) as *const CapabilityInfo };

        for i in 0..entry_count {
            let capability_info = unsafe { ptr::read(entries_ptr.add(i)) };
            result.push(capability_info);
        }

        Ok(result)
    }

    /// Calculates the total size needed to store the given capabilities.
    fn calculate_size(capabilities: &[CapabilityInfo]) -> usize {
        Self::calculate_size_for_count(capabilities.len())
    }

    /// Calculates the total size needed to store the given number of capabilities.
    fn calculate_size_for_count(count: usize) -> usize {
        mem::size_of::<Header>() + count * mem::size_of::<CapabilityInfo>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityBlockReadError {
    InvalidVersion,
}

impl fmt::Display for CapabilityBlockReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersion => write!(f, "Invalid CapabilityBlock version"),
        }
    }
}

impl error::Error for CapabilityBlockReadError {}

#[derive(Debug)]
#[repr(C)]
struct Header {
    pub version: u32,
    pub entry_count: u32,
}
