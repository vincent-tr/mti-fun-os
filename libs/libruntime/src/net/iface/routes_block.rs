use alloc::{string::String, vec::Vec};
use core::{fmt, mem, ptr, slice};

use crate::{
    memory::align_up,
    net::types::{IpAddress, IpPrefix},
};

/// Version of the RoutesBlock format.
const VERSION: u32 = 1;

/// Route.
#[derive(Debug, Clone)]
pub struct Route {
    /// Network
    pub prefix: IpPrefix,

    /// Optional gateway
    pub gateway: Option<IpAddress>,

    /// Interface used
    pub iface: String,

    /// Metric for this route
    pub metric: usize,
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.prefix.len() == 0 {
            write!(f, "default")?;
        } else {
            write!(f, "{}", self.prefix)?;
        }

        if let Some(gateway) = self.gateway {
            write!(f, " via {}", gateway)?;
        }

        write!(f, " dev {}", self.iface)?;

        if self.metric != 0 {
            write!(f, " metric {}", self.metric)?;
        }

        Ok(())
    }
}

/// Routes block format for IPC communication.
pub struct RoutesBlock;

impl RoutesBlock {
    /// Builds a routes list into the provided buffer.
    ///
    /// Returns `Ok(bytes_written)` on success, or `Err(required_size)` if the buffer is too small.
    pub fn build(routes: &[Route], buffer: &mut [u8]) -> Result<usize, usize> {
        let required_size = Self::calculate_size(routes);
        if buffer.len() < required_size {
            return Err(required_size);
        }

        // Write header
        let header = Header {
            version: VERSION,
            entry_count: routes.len() as u32,
        };
        unsafe {
            ptr::write(buffer.as_mut_ptr() as *mut Header, header);
        }

        let mut offset = align_up(mem::size_of::<Header>(), mem::align_of::<RouteEntry>());

        for route in routes {
            // Write entry
            let entry = RouteEntry {
                prefix: route.prefix,
                gateway: route.gateway,
                iface_len: route.iface.len() as u32,
                metric: route.metric,
            };
            unsafe {
                ptr::write(buffer[offset..].as_mut_ptr() as *mut RouteEntry, entry);
            }
            offset += mem::size_of::<RouteEntry>();

            // Write iface
            let iface_bytes = route.iface.as_bytes();
            buffer[offset..offset + iface_bytes.len()].copy_from_slice(iface_bytes);
            offset += iface_bytes.len();

            // Align for next entry
            offset = align_up(offset, mem::align_of::<RouteEntry>());
        }

        Ok(required_size)
    }

    /// Reads a routes list from the provided buffer.
    ///
    /// Returns a vector of Route on success, or an error if we get the wrong version.
    pub fn read(buffer: &[u8]) -> Result<Vec<Route>, RoutesBlockReadError> {
        assert!(buffer.len() >= mem::size_of::<Header>());

        // Read header
        let header = unsafe { &*(buffer.as_ptr() as *const Header) };

        if header.version != VERSION {
            return Err(RoutesBlockReadError::InvalidVersion);
        }

        let mut result = Vec::with_capacity(header.entry_count as usize);
        let mut offset = align_up(mem::size_of::<Header>(), mem::align_of::<RouteEntry>());

        for _ in 0..header.entry_count {
            assert!(offset + mem::size_of::<RouteEntry>() <= buffer.len());

            let entry = unsafe { &*(buffer[offset..].as_ptr() as *const RouteEntry) };

            assert!(offset + entry.total_size() <= buffer.len());

            let route = unsafe { entry.to_route() };
            result.push(route);

            offset += align_up(entry.total_size(), mem::align_of::<RouteEntry>());
        }

        Ok(result)
    }

    /// Calculates the total size needed to store the given routes.
    fn calculate_size(routes: &[Route]) -> usize {
        let mut total_size = mem::size_of::<Header>();
        for route in routes {
            total_size = align_up(total_size, mem::align_of::<RouteEntry>());
            total_size += mem::size_of::<RouteEntry>();
            total_size += route.iface.len();
        }
        total_size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutesBlockReadError {
    InvalidVersion,
}

impl core::fmt::Display for RoutesBlockReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidVersion => write!(f, "Invalid RoutesBlock version"),
        }
    }
}

impl core::error::Error for RoutesBlockReadError {}

#[derive(Debug)]
#[repr(C)]
struct Header {
    pub version: u32,
    pub entry_count: u32,
}

#[derive(Debug)]
#[repr(C)]
struct RouteEntry {
    pub prefix: IpPrefix,
    pub gateway: Option<IpAddress>,
    pub iface_len: u32,
    pub metric: usize,
}

impl RouteEntry {
    pub fn total_size(&self) -> usize {
        mem::size_of::<RouteEntry>() + self.iface_len as usize
    }

    /// Safety: The caller must ensure that the RouteEntry is valid and followed by valid string data.
    pub unsafe fn to_route(&self) -> Route {
        let entry_ptr = self as *const RouteEntry as usize;
        let iface_start = entry_ptr + mem::size_of::<RouteEntry>();

        let iface_bytes =
            unsafe { slice::from_raw_parts(iface_start as *const u8, self.iface_len as usize) };
        let iface = unsafe { core::str::from_utf8_unchecked(iface_bytes) };

        Route {
            prefix: self.prefix,
            gateway: self.gateway,
            iface: String::from(iface),
            metric: self.metric,
        }
    }
}
