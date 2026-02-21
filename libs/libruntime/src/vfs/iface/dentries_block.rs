use alloc::{string::String, vec::Vec};
use core::{mem, ptr, slice};

use crate::memory::align_up;
use crate::vfs::types::NodeType;

/// Version of the DentriesBlock format.
const VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// Name of the entry, e.g. "file.txt"
    pub name: String,

    /// Type of the entry (file, directory, or symlink).
    pub r#type: NodeType,
}

/// Directory entries block format for IPC communication.
pub struct DentriesBlock;

impl DentriesBlock {
    /// Builds a directory entries list into the provided buffer.
    ///
    /// Returns `Ok(bytes_written)` on success, or `Err(required_size)` if the buffer is too small.
    pub fn build(entries: &[DirectoryEntry], buffer: &mut [u8]) -> Result<usize, usize> {
        let required_size = Self::calculate_size(entries);
        if buffer.len() < required_size {
            return Err(required_size);
        }

        // Write header
        let header = Header {
            version: VERSION,
            entry_count: entries.len() as u32,
        };
        unsafe {
            ptr::write(buffer.as_mut_ptr() as *mut Header, header);
        }

        let mut offset = align_up(mem::size_of::<Header>(), mem::align_of::<DentryEntry>());

        for entry in entries {
            // Write entry
            let dentry = DentryEntry {
                r#type: entry.r#type,
                name_len: entry.name.len() as u32,
            };
            unsafe {
                ptr::write(buffer[offset..].as_mut_ptr() as *mut DentryEntry, dentry);
            }
            offset += mem::size_of::<DentryEntry>();

            // Write name
            let name_bytes = entry.name.as_bytes();
            buffer[offset..offset + name_bytes.len()].copy_from_slice(name_bytes);
            offset += name_bytes.len();

            // Align for next entry
            offset = align_up(offset, mem::align_of::<DentryEntry>());
        }

        Ok(required_size)
    }

    /// Reads a directory entries list from the provided buffer.
    ///
    /// Returns a vector of DirectoryEntry on success, or an error if we get the wrong version.
    pub fn read(buffer: &[u8]) -> Result<Vec<DirectoryEntry>, DentriesBlockReadError> {
        assert!(buffer.len() >= mem::size_of::<Header>());

        // Read header
        let header = unsafe { &*(buffer.as_ptr() as *const Header) };

        if header.version != VERSION {
            return Err(DentriesBlockReadError::InvalidVersion);
        }

        let mut result = Vec::with_capacity(header.entry_count as usize);
        let mut offset = align_up(mem::size_of::<Header>(), mem::align_of::<DentryEntry>());

        for _ in 0..header.entry_count {
            assert!(offset + mem::size_of::<DentryEntry>() <= buffer.len());

            let entry = unsafe { &*(buffer[offset..].as_ptr() as *const DentryEntry) };

            assert!(offset + entry.total_size() <= buffer.len());

            let directory_entry = unsafe { entry.to_directory_entry() };
            result.push(directory_entry);

            offset += align_up(entry.total_size(), mem::align_of::<DentryEntry>());
        }

        Ok(result)
    }

    /// Calculates the total size needed to store the given directory entries.
    fn calculate_size(entries: &[DirectoryEntry]) -> usize {
        let mut total_size = mem::size_of::<Header>();
        for entry in entries {
            total_size = align_up(total_size, mem::align_of::<DentryEntry>());
            total_size += mem::size_of::<DentryEntry>();
            total_size += entry.name.len();
        }
        total_size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentriesBlockReadError {
    InvalidVersion,
}

impl core::fmt::Display for DentriesBlockReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidVersion => write!(f, "Invalid DentriesBlock version"),
        }
    }
}

impl core::error::Error for DentriesBlockReadError {}

#[derive(Debug)]
#[repr(C)]
struct Header {
    pub version: u32,
    pub entry_count: u32,
}

#[derive(Debug)]
#[repr(C)]
struct DentryEntry {
    pub r#type: NodeType,
    pub name_len: u32,
}

impl DentryEntry {
    pub fn total_size(&self) -> usize {
        mem::size_of::<DentryEntry>() + self.name_len as usize
    }

    /// Safety: The caller must ensure that the DentryEntry is valid and followed by valid string data.
    pub unsafe fn to_directory_entry(&self) -> DirectoryEntry {
        let entry_ptr = self as *const DentryEntry as usize;
        let name_start = entry_ptr + mem::size_of::<DentryEntry>();

        let name_bytes =
            unsafe { slice::from_raw_parts(name_start as *const u8, self.name_len as usize) };
        let name = unsafe { core::str::from_utf8_unchecked(name_bytes) };

        DirectoryEntry {
            name: String::from(name),
            r#type: self.r#type,
        }
    }
}
