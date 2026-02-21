use alloc::{string::String, vec::Vec};
use core::{mem, ptr, slice};

use crate::memory::align_up;

use super::messages;

/// Version of the ProcessListBlock format.
const VERSION: u32 = 1;

/// Information about a process, as usable format
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Process ID.
    pub pid: u64,

    /// Parent Process ID.
    pub ppid: u64,

    /// Process name.
    pub name: String,

    /// Process status.
    pub status: messages::ProcessStatus,
}

/// Process list block format for IPC communication.
pub struct ProcessListBlock;

impl ProcessListBlock {
    /// Builds a process list into the provided buffer.
    ///
    /// Returns `Ok(bytes_written)` on success, or `Err(required_size)` if the buffer is too small.
    pub fn build(processes: &[ProcessInfo], buffer: &mut [u8]) -> Result<usize, usize> {
        let required_size = Self::calculate_size(processes);
        if buffer.len() < required_size {
            return Err(required_size);
        }

        // Write header
        let header = Header {
            version: VERSION,
            entry_count: processes.len() as u32,
        };
        unsafe {
            ptr::write(buffer.as_mut_ptr() as *mut Header, header);
        }

        let mut offset = align_up(mem::size_of::<Header>(), mem::align_of::<ProcessEntry>());

        for process in processes {
            // Write entry
            let entry = ProcessEntry {
                pid: process.pid,
                ppid: process.ppid,
                status: process.status,
                name_len: process.name.len() as u32,
            };
            unsafe {
                ptr::write(buffer[offset..].as_mut_ptr() as *mut ProcessEntry, entry);
            }
            offset += mem::size_of::<ProcessEntry>();

            // Write name
            let name_bytes = process.name.as_bytes();
            buffer[offset..offset + name_bytes.len()].copy_from_slice(name_bytes);
            offset += name_bytes.len();

            // Align for next entry
            offset = align_up(offset, mem::align_of::<ProcessEntry>());
        }

        Ok(required_size)
    }

    /// Reads a process list from the provided buffer.
    ///
    /// Returns a vector of ProcessInfo on success, or an error if we get the wrong version.
    pub fn read(buffer: &[u8]) -> Result<Vec<ProcessInfo>, PLBlockReadError> {
        assert!(buffer.len() >= mem::size_of::<Header>());

        // Read header
        let header = unsafe { &*(buffer.as_ptr() as *const Header) };

        if header.version != VERSION {
            return Err(PLBlockReadError::InvalidVersion);
        }

        let mut result = Vec::with_capacity(header.entry_count as usize);
        let mut offset = align_up(mem::size_of::<Header>(), mem::align_of::<ProcessEntry>());

        for _ in 0..header.entry_count {
            assert!(offset + mem::size_of::<ProcessEntry>() <= buffer.len());

            let entry = unsafe { &*(buffer[offset..].as_ptr() as *const ProcessEntry) };

            assert!(offset + entry.total_size() <= buffer.len());

            let process_info = unsafe { entry.to_process_info() };
            result.push(process_info);

            offset += align_up(entry.total_size(), mem::align_of::<ProcessEntry>());
        }

        Ok(result)
    }

    /// Calculates the total size needed to store the given processes.
    fn calculate_size(processes: &[ProcessInfo]) -> usize {
        let mut total_size = mem::size_of::<Header>();
        for process in processes {
            total_size = align_up(total_size, mem::align_of::<ProcessEntry>());
            total_size += mem::size_of::<ProcessEntry>();
            total_size += process.name.len();
        }
        total_size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PLBlockReadError {
    InvalidVersion,
}

impl core::fmt::Display for PLBlockReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidVersion => write!(f, "Invalid ProcessListBlock version"),
        }
    }
}

impl core::error::Error for PLBlockReadError {}

#[derive(Debug)]
#[repr(C)]
struct Header {
    pub version: u32,
    pub entry_count: u32,
}

#[derive(Debug)]
#[repr(C)]
struct ProcessEntry {
    pub pid: u64,
    pub ppid: u64,
    pub status: super::messages::ProcessStatus,
    pub name_len: u32,
}

impl ProcessEntry {
    pub fn total_size(&self) -> usize {
        mem::size_of::<ProcessEntry>() + self.name_len as usize
    }

    /// Safety: The caller must ensure that the ProcessEntry is valid and followed by valid string data.
    pub unsafe fn to_process_info(&self) -> ProcessInfo {
        let name_start = (self as *const ProcessEntry as usize) + mem::size_of::<ProcessEntry>();
        let name_bytes =
            unsafe { slice::from_raw_parts(name_start as *const u8, self.name_len as usize) };
        let name = unsafe { core::str::from_utf8_unchecked(name_bytes) };

        ProcessInfo {
            pid: self.pid,
            ppid: self.ppid,
            status: self.status,
            name: alloc::string::String::from(name),
        }
    }
}
