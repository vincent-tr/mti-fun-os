use core::ops::Range;

use alloc::{collections::BTreeSet, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;
use syscalls::Error;
use x86_64::instructions::port::Port;

pub use syscalls::PortAccess;

/// Represents a range of I/O ports with specific access rights.
#[derive(Debug)]
pub struct PortRange {
    range: Range<u16>,
    access: PortAccess,
}

impl Drop for PortRange {
    fn drop(&mut self) {
        RESERVATIONS.lock().remove(&self.range);
    }
}

impl PortRange {
    pub fn new(from: usize, count: usize, access: PortAccess) -> Result<Arc<Self>, Error> {
        let end = from + count;
        if from > u16::MAX as usize || end > u16::MAX as usize {
            return Err(Error::InvalidArgument);
        }

        let range = (from as u16)..(end as u16);

        if !RESERVATIONS.lock().add(range.clone()) {
            return Err(Error::MemoryAccessDenied);
        }

        Ok(Arc::new(Self { range, access }))
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
        log::debug!("PortRange read: index={}, word_size={}", index, word_size);
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

lazy_static! {
    /// Global list of reserved port ranges to prevent overlapping allocations.
    static ref RESERVATIONS: Mutex<ReservedRangeList> = Mutex::new(ReservedRangeList::new());
}

/// A collection of reserved port ranges that prevents overlapping allocations.
#[derive(Debug)]
pub struct ReservedRangeList {
    // Use (start, end) tuple as key since Range doesn't implement Ord
    ranges: BTreeSet<(u16, u16)>,
}

impl ReservedRangeList {
    /// Creates a new empty reserved range list.
    pub fn new() -> Self {
        Self {
            ranges: BTreeSet::new(),
        }
    }

    /// Adds a new port range to the list.
    /// Returns false if the range overlaps with any existing range, true if successfully added.
    pub fn add(&mut self, range: Range<u16>) -> bool {
        if self.has_overlap(&range) {
            return false;
        }

        self.ranges.insert((range.start, range.end));
        true
    }

    /// Checks if the given port range overlaps with any existing range in the list.
    fn has_overlap(&self, range: &Range<u16>) -> bool {
        // Uses BTreeSet ordering for efficient O(log n) lookup.
        let start = range.start;
        let end = range.end;

        // Check the range at or after our start position
        // If it starts before our end, they overlap
        if let Some((existing_start, _)) = self.ranges.range((start, 0)..).next() {
            if *existing_start < end {
                return true;
            }
        }

        // Check the range immediately before our start position
        // If it ends after our start, they overlap
        if let Some((_, existing_end)) = self.ranges.range(..(start, 0)).next_back() {
            if *existing_end > start {
                return true;
            }
        }

        false
    }

    /// Removes a port range from the list.
    /// Returns true if the range was found and removed, false otherwise.
    pub fn remove(&mut self, range: &Range<u16>) -> bool {
        self.ranges.remove(&(range.start, range.end))
    }
}
