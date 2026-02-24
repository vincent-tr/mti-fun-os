use core::{mem, ops::Range};

use crate::memory::{FrameRef, PAGE_SIZE, access_phys, is_page_aligned, phys_allocate};
use alloc::{collections::btree_set::BTreeSet, sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::PhysAddr;

use super::{Error, error::*};

/// Represent a area in physical or device memory, that can be mapped into processes
#[derive(Debug)]
pub struct MemoryObject(Type);

impl MemoryObject {
    /// Create a new memory object of the given size
    pub fn new(size: usize) -> Result<Arc<Self>, Error> {
        Ok(Arc::new(Self(Type::Physical(PhysicalMemory::new(size)?))))
    }

    pub fn new_iomem(address: PhysAddr, size: usize) -> Result<Arc<Self>, Error> {
        // TODO: check ranges
        Ok(Arc::new(Self(Type::Device(DeviceMemory::new(
            address, size,
        )?))))
    }

    /// Get the size of the memory object
    pub fn size(&self) -> usize {
        self.0.frame_count() * PAGE_SIZE
    }

    /// Get a particular frame of the memory object
    pub fn frame(&self, offset: usize) -> PhysAddr {
        assert!(is_page_aligned(offset));
        assert!(offset < self.size());
        let index = offset / PAGE_SIZE;

        self.0.frame(index)
    }

    /// Borrow a particular frame of the memory object, preventing it from being reallocated until unborrowed
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `borrow()` call match a `unborrow()` call.
    pub unsafe fn borrow_frame(&self, offset: usize) -> PhysAddr {
        assert!(is_page_aligned(offset));
        assert!(offset < self.size());
        let index = offset / PAGE_SIZE;

        unsafe { self.0.borrow_frame(index) }
    }

    /// Unborrow a particular frame of the memory object, decreasing its borrow count
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `unborrow()` call matchs `borrow()` call.
    pub unsafe fn unborrow_frame(&self, frame: PhysAddr) {
        unsafe { self.0.unborrow_frame(frame) }
    }

    /// Iterates over the frames of the memory object
    pub fn frames_iter(&self) -> impl Iterator<Item = PhysAddr> + '_ {
        (0..self.0.frame_count()).map(|i| self.0.frame(i))
    }
}

/// Type of memory object, either physical memory or device memory
#[derive(Debug)]
enum Type {
    Physical(PhysicalMemory),
    Device(DeviceMemory),
}

impl Type {
    /// Get the count of frames in the memory object
    pub fn frame_count(&self) -> usize {
        match self {
            Type::Physical(phys) => phys.frame_count(),
            Type::Device(device) => device.frame_count(),
        }
    }

    /// Get a particular frame of he memory object
    pub fn frame(&self, index: usize) -> PhysAddr {
        match self {
            Type::Physical(phys) => phys.frame(index),
            Type::Device(device) => device.frame(index),
        }
    }

    /// Borrow a particular frame of the memory object, preventing it from being reallocated until unborrowed
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `borrow()` call match a `unborrow()` call.
    pub unsafe fn borrow_frame(&self, index: usize) -> PhysAddr {
        match self {
            Type::Physical(phys) => unsafe { phys.borrow_frame(index) },
            Type::Device(device) => unsafe { device.borrow_frame(index) },
        }
    }

    /// Unborrow a particular frame of the memory object, decreasing its borrow count
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `unborrow()` call matchs `borrow()` call.
    pub unsafe fn unborrow_frame(&self, phys_addr: PhysAddr) {
        match self {
            Type::Physical(phys) => unsafe { phys.unborrow_frame(phys_addr) },
            Type::Device(device) => unsafe { device.unborrow_frame(phys_addr) },
        }
    }
}

/// A memory object that represents a region of physical memory
#[derive(Debug)]
struct PhysicalMemory {
    /// The list of physical frames that back the memory object
    ///
    /// The PhysicalMemory object owns the frames, and they will get back to the frame pool when the object is dropped
    pages: Vec<FrameRef>,
}

impl PhysicalMemory {
    /// Create a new memory object of the given size
    pub fn new(size: usize) -> Result<Self, Error> {
        check_page_alignment(size)?;
        check_positive(size)?;

        let page_count = size / PAGE_SIZE;
        let mut object = Self {
            pages: Vec::with_capacity(page_count),
        };

        for _ in 0..page_count {
            match phys_allocate() {
                Some(frame) => {
                    object.pages.push(frame);
                }

                None => {
                    // Dropping the list of pages will drop all frames created so far
                    return Err(out_of_memory());
                }
            }
        }

        for page in object.pages.iter() {
            Self::zero_page(page);
        }

        Ok(object)
    }

    fn zero_page(page: &FrameRef) {
        let page_data = unsafe { access_phys(page.frame()) };
        page_data.fill(0);
    }

    /// Get the count of physical frames in the memory object
    pub fn frame_count(&self) -> usize {
        self.pages.len()
    }

    /// Get a particular physical frame of he memory object
    pub fn frame(&self, index: usize) -> PhysAddr {
        self.pages[index].frame()
    }

    /// Borrow a particular physical frame of the memory object, preventing it from being reallocated until unborrowed
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `borrow()` call match a `unborrow()` call.
    pub unsafe fn borrow_frame(&self, index: usize) -> PhysAddr {
        let mut frame = self.pages[index].clone();
        // Mark it as used
        unsafe { frame.borrow() }
    }

    /// Unborrow a particular physical frame of the memory object, decreasing its borrow count
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `unborrow()` call matchs `borrow()` call.
    pub unsafe fn unborrow_frame(&self, phys_addr: PhysAddr) {
        let frame = unsafe { FrameRef::unborrow(phys_addr) };
        mem::drop(frame);
    }
}

/// A memory object that represents a region of device memory
#[derive(Debug)]
struct DeviceMemory {
    /// The physical address of the start of the device memory region
    address: PhysAddr,

    /// The number of frames in the device memory region
    frames: usize,
}

impl DeviceMemory {
    /// Create a new memory object that represents the given region of device memory
    pub fn new(address: PhysAddr, size: usize) -> Result<Self, Error> {
        check_page_alignment(address.as_u64() as usize)?;
        check_page_alignment(size)?;
        check_positive(size)?;

        let frames = size / PAGE_SIZE;
        if !DEVICE_RESERVATIONS.lock().add(Self::range(address, frames)) {
            return Err(Error::MemoryAccessDenied);
        }

        Ok(Self { address, frames })
    }

    fn range(address: PhysAddr, frames: usize) -> Range<u64> {
        let start = address.as_u64();
        let end = start + (frames * PAGE_SIZE) as u64;
        start..end
    }

    /// Get the count of device memory frames in the memory object
    pub fn frame_count(&self) -> usize {
        self.frames
    }

    /// Get a particular device memory frame of he memory object
    pub fn frame(&self, index: usize) -> PhysAddr {
        assert!(index < self.frames);
        self.address + (index * PAGE_SIZE) as u64
    }

    /// Borrow a particular device memory frame of the memory object
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `borrow()` call match a `unborrow()` call.
    pub unsafe fn borrow_frame(&self, index: usize) -> PhysAddr {
        // For device memory, we don't need to track borrows since they can't be reallocated
        self.frame(index)
    }

    /// Unborrow a particular device memory frame of the memory object
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `unborrow()` call matchs `borrow()` call.
    pub unsafe fn unborrow_frame(&self, _phys_addr: PhysAddr) {
        // No-op for device memory
    }
}

impl Drop for DeviceMemory {
    fn drop(&mut self) {
        let range = Self::range(self.address, self.frames);
        DEVICE_RESERVATIONS.lock().remove(range);
    }
}

lazy_static! {
    /// Global list of reserved device memory ranges to prevent overlapping allocations.
    static ref DEVICE_RESERVATIONS: Mutex<DeviceReservedRangeList> = Mutex::new(DeviceReservedRangeList::new());
}

/// A collection of reserved device memory ranges that prevents overlapping allocations.
#[derive(Debug)]
pub struct DeviceReservedRangeList {
    // Use (start, end) tuple as key since Range doesn't implement Ord
    ranges: BTreeSet<(u64, u64)>,
}

impl DeviceReservedRangeList {
    /// Creates a new empty reserved range list.
    pub fn new() -> Self {
        Self {
            ranges: BTreeSet::new(),
        }
    }

    /// Adds a new device memory range to the list.
    /// Returns false if the range overlaps with any existing range, true if successfully added.
    pub fn add(&mut self, range: Range<u64>) -> bool {
        if self.has_overlap(&range) {
            return false;
        }

        self.ranges.insert((range.start, range.end));
        true
    }

    /// Checks if the given device memory range overlaps with any existing range in the list.
    fn has_overlap(&self, range: &Range<u64>) -> bool {
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

    /// Removes a device memory range from the list.
    /// Returns true if the range was found and removed, false otherwise.
    pub fn remove(&mut self, range: Range<u64>) -> bool {
        self.ranges.remove(&(range.start, range.end))
    }
}
