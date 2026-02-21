use alloc::{string::String, vec::Vec};
use core::{mem, ptr, slice};

use crate::memory::align_up;

/// Version of the MountsBlock format.
const VERSION: u32 = 1;

/// Information about a mounted filesystem, used in the ListMounts message.
#[derive(Debug, Clone)]
pub struct MountInfo {
    /// Mount point path, e.g. "/mnt/usb"
    pub mount_point: String,

    /// Port name of the filesystem driver, e.g. "usbfs"
    pub fs_port_name: String,
}

/// Mounts block format for IPC communication.
pub struct MountsBlock;

impl MountsBlock {
    /// Builds a mounts list into the provided buffer.
    ///
    /// Returns `Ok(bytes_written)` on success, or `Err(required_size)` if the buffer is too small.
    pub fn build(mounts: &[MountInfo], buffer: &mut [u8]) -> Result<usize, usize> {
        let required_size = Self::calculate_size(mounts);
        if buffer.len() < required_size {
            return Err(required_size);
        }

        // Write header
        let header = Header {
            version: VERSION,
            entry_count: mounts.len() as u32,
        };
        unsafe {
            ptr::write(buffer.as_mut_ptr() as *mut Header, header);
        }

        let mut offset = align_up(mem::size_of::<Header>(), mem::align_of::<MountEntry>());

        for mount in mounts {
            // Write entry
            let entry = MountEntry {
                mount_point_len: mount.mount_point.len() as u32,
                fs_port_name_len: mount.fs_port_name.len() as u32,
            };
            unsafe {
                ptr::write(buffer[offset..].as_mut_ptr() as *mut MountEntry, entry);
            }
            offset += mem::size_of::<MountEntry>();

            // Write mount_point
            let mount_point_bytes = mount.mount_point.as_bytes();
            buffer[offset..offset + mount_point_bytes.len()].copy_from_slice(mount_point_bytes);
            offset += mount_point_bytes.len();

            // Write fs_port_name
            let fs_port_name_bytes = mount.fs_port_name.as_bytes();
            buffer[offset..offset + fs_port_name_bytes.len()].copy_from_slice(fs_port_name_bytes);
            offset += fs_port_name_bytes.len();

            // Align for next entry
            offset = align_up(offset, mem::align_of::<MountEntry>());
        }

        Ok(required_size)
    }

    /// Reads a mounts list from the provided buffer.
    ///
    /// Returns a vector of MountInfo on success, or an error if we get the wrong version.
    pub fn read(buffer: &[u8]) -> Result<Vec<MountInfo>, MountsBlockReadError> {
        assert!(buffer.len() >= mem::size_of::<Header>());

        // Read header
        let header = unsafe { &*(buffer.as_ptr() as *const Header) };

        if header.version != VERSION {
            return Err(MountsBlockReadError::InvalidVersion);
        }

        let mut result = Vec::with_capacity(header.entry_count as usize);
        let mut offset = align_up(mem::size_of::<Header>(), mem::align_of::<MountEntry>());

        for _ in 0..header.entry_count {
            assert!(offset + mem::size_of::<MountEntry>() <= buffer.len());

            let entry = unsafe { &*(buffer[offset..].as_ptr() as *const MountEntry) };

            assert!(offset + entry.total_size() <= buffer.len());

            let mount_info = unsafe { entry.to_mount_info() };
            result.push(mount_info);

            offset += align_up(entry.total_size(), mem::align_of::<MountEntry>());
        }

        Ok(result)
    }

    /// Calculates the total size needed to store the given mounts.
    fn calculate_size(mounts: &[MountInfo]) -> usize {
        let mut total_size = mem::size_of::<Header>();
        for mount in mounts {
            total_size = align_up(total_size, mem::align_of::<MountEntry>());
            total_size += mem::size_of::<MountEntry>();
            total_size += mount.mount_point.len();
            total_size += mount.fs_port_name.len();
        }
        total_size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountsBlockReadError {
    InvalidVersion,
}

impl core::fmt::Display for MountsBlockReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidVersion => write!(f, "Invalid MountsBlock version"),
        }
    }
}

impl core::error::Error for MountsBlockReadError {}

#[derive(Debug)]
#[repr(C)]
struct Header {
    pub version: u32,
    pub entry_count: u32,
}

#[derive(Debug)]
#[repr(C)]
struct MountEntry {
    pub mount_point_len: u32,
    pub fs_port_name_len: u32,
}

impl MountEntry {
    pub fn total_size(&self) -> usize {
        mem::size_of::<MountEntry>()
            + self.mount_point_len as usize
            + self.fs_port_name_len as usize
    }

    /// Safety: The caller must ensure that the MountEntry is valid and followed by valid string data.
    pub unsafe fn to_mount_info(&self) -> MountInfo {
        let entry_ptr = self as *const MountEntry as usize;
        let mount_point_start = entry_ptr + mem::size_of::<MountEntry>();
        let fs_port_name_start = mount_point_start + self.mount_point_len as usize;

        let mount_point_bytes = unsafe {
            slice::from_raw_parts(
                mount_point_start as *const u8,
                self.mount_point_len as usize,
            )
        };
        let mount_point = unsafe { core::str::from_utf8_unchecked(mount_point_bytes) };

        let fs_port_name_bytes = unsafe {
            slice::from_raw_parts(
                fs_port_name_start as *const u8,
                self.fs_port_name_len as usize,
            )
        };
        let fs_port_name = unsafe { core::str::from_utf8_unchecked(fs_port_name_bytes) };

        MountInfo {
            mount_point: String::from(mount_point),
            fs_port_name: String::from(fs_port_name),
        }
    }
}
