use core::{
    mem::size_of,
    ptr::{self, slice_from_raw_parts_mut},
};

use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use log::info;
use spin::RwLock;
use x86_64::{PhysAddr, VirtAddr};

use super::PAGE_SIZE;

#[derive(Debug)]
#[repr(C)]
struct Descriptor {
    ref_count: usize,
    prev: *mut Descriptor,
    next: *mut Descriptor,
}

impl Descriptor {
    const fn new() -> Self {
        Self {
            ref_count: 0,
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }

    fn r#ref(&mut self) {
        self.ref_count += 1;
    }

    /// Returns true if ref_count > 0, false if ref_count = 0 after the operation
    fn unref(&mut self) -> bool {
        debug_assert!(self.ref_count > 0);
        self.ref_count -= 1;
        self.ref_count > 0
    }

    fn used(&self) -> bool {
        self.ref_count > 0
    }
}

struct List {
    head: *mut Descriptor,
    count: usize,
}

impl List {
    const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
            count: 0,
        }
    }

    unsafe fn add(&mut self, desc: *mut Descriptor) {
        debug_assert!(!desc.is_null());
        debug_assert!((*desc).prev.is_null() && (*desc).next.is_null());
        debug_assert!(!self.has(desc));

        if self.head.is_null() {
            (*desc).next = desc;
            (*desc).prev = desc;
            self.head = desc;
        } else {
            // insert after head
            let prev = self.head;
            let next = (*self.head).next;
            (*prev).next = desc;
            (*desc).prev = prev;
            (*next).prev = desc;
            (*desc).next = next;
        }

        self.count += 1;
    }

    unsafe fn remove(&mut self, desc: *mut Descriptor) {
        debug_assert!(!desc.is_null());
        debug_assert!(!(*desc).prev.is_null() && !(*desc).next.is_null());
        debug_assert!(self.has(desc));

        let prev = (*desc).prev;
        let next = (*desc).next;

        if desc == prev {
            // if we had only one item
            self.head = ptr::null_mut();
        } else if prev == next {
            // if we had 2 items, now 1
            let item = prev;
            (*item).next = item;
            (*item).prev = item;
            self.head = item;
        } else {
            // normal case
            (*prev).next = next;
            (*next).prev = prev;
            if self.head == desc {
                self.head = next;
            }
        }

        (*desc).prev = ptr::null_mut();
        (*desc).next = ptr::null_mut();
        self.count -= 1;
    }

    unsafe fn has(&self, desc: *mut Descriptor) -> bool {
        if self.head.is_null() {
            return false;
        }

        let mut item = self.head;

        loop {
            if item == desc {
                return true;
            }

            item = (*item).next;

            if item == self.head {
                return false;
            }
        }
    }
}

struct Allocator {
    descriptors: *mut [Descriptor],
    used_list: List,
    free_list: List,
}

unsafe impl Sync for Allocator {}
unsafe impl Send for Allocator {}

impl Allocator {
    pub const fn new() -> Self {
        Self {
            descriptors: slice_from_raw_parts_mut(ptr::null_mut(), 0),
            used_list: List::new(),
            free_list: List::new(),
        }
    }

    pub const fn needed_buffer_size(page_count: usize) -> usize {
        page_count * size_of::<Descriptor>()
    }

    unsafe fn init_descriptors(&mut self, buffer: VirtAddr, buffer_size: usize) {
        assert!(buffer.is_aligned(PAGE_SIZE as u64));
        assert!((buffer + buffer_size).is_aligned(PAGE_SIZE as u64));

        let data: *mut Descriptor = buffer.as_mut_ptr();
        let count = buffer_size / size_of::<Descriptor>();

        self.descriptors = slice_from_raw_parts_mut(data, count);

        // initialize descriptors
        // make all pages as used by default
        let descs = &mut (*self.descriptors);
        for desc in descs {
            *desc = Descriptor::new();

            self.used_list.add(desc);
            desc.r#ref();
        }
    }

    unsafe fn allocate(&mut self) -> Result<PhysAddr, AllocatorError> {
        let desc = self.free_list.head;

        if desc.is_null() {
            return Err(AllocatorError::NoMemory);
        }

        self.free_list.remove(desc);
        self.used_list.add(desc);
        let desc_ref = &mut (*desc);

        desc_ref.r#ref();

        Ok(self.desc_to_frame(desc_ref))
    }

    unsafe fn allocate_at(&mut self, frame: PhysAddr) -> Result<(), AllocatorError> {
        let desc = self.frame_to_desc(frame);
        let desc_ref = &mut (*desc);

        if desc_ref.used() {
            return Err(AllocatorError::NoMemory);
        }

        self.free_list.remove(desc);
        self.used_list.add(desc);
        let desc_ref = &mut (*desc);

        desc_ref.r#ref();

        Ok(())
    }

    unsafe fn r#ref(&mut self, frame: PhysAddr) {
        let desc = self.frame_to_desc(frame);
        let desc_ref = &mut (*desc);

        desc_ref.r#ref();
    }

    /// Returns true if the page has still references, false if it has been deallocated
    unsafe fn unref(&mut self, frame: PhysAddr) -> bool {
        let desc = self.frame_to_desc(frame);
        let desc_ref = &mut (*desc);

        debug_assert!(desc_ref.used(), "Unref unused frame {:?}", frame);

        let has_ref = desc_ref.unref();

        if !has_ref {
            self.used_list.remove(desc);
            self.free_list.add(desc);
        }

        has_ref
    }

    unsafe fn frame_to_desc(&self, frame: PhysAddr) -> *mut Descriptor {
        let index = frame.as_u64() as usize / PAGE_SIZE;
        self.descriptors.as_mut_ptr().add(index)
    }

    unsafe fn desc_to_frame(&self, desc: *mut Descriptor) -> PhysAddr {
        let index = desc.sub_ptr(self.descriptors.as_mut_ptr());
        PhysAddr::new((index * PAGE_SIZE)  as u64)
    }

    fn check_frame(&self, frame: PhysAddr) -> bool {
        return frame.is_aligned(PAGE_SIZE as u64)
            && frame < PhysAddr::new((self.descriptors.len() * PAGE_SIZE)  as u64);
    }
}

#[derive(Debug)]
pub enum AllocatorError {
    NoMemory,
}

static ALLOCATOR: RwLock<Allocator> = RwLock::new(Allocator::new());

pub fn init(phys_mapping: VirtAddr, memory_regions: &MemoryRegions) {
    // memory regions looks ordered.
    assert!(memory_regions.is_sorted_by_key(|region| { region.start }));

    let end = memory_regions.last().unwrap().end;
    assert!(PhysAddr::new(end).is_aligned(PAGE_SIZE as u64));

    let count = end as usize / PAGE_SIZE;
    let buffer_size = Allocator::needed_buffer_size(count);
    let buffer_phys = find_usable_region(memory_regions, buffer_size);
    let buffer_phys_end = buffer_phys + buffer_size;
    let buffer: VirtAddr = phys_mapping + buffer_phys.as_u64();

    info!(
        "Using physical frame descriptors at {:?} (frame count={})",
        buffer, count
    );

    {
        let mut allocator = ALLOCATOR.write();
        unsafe {
            allocator.init_descriptors(buffer, buffer_size);
        }

        // Note: by default all pages are created as reserved

        for memory_region in memory_regions.iter() {
            if memory_region.kind != MemoryRegionKind::Usable {
                continue;
            }

            let mut frame = PhysAddr::new(memory_region.start);
            while frame < PhysAddr::new(memory_region.end) {
                if frame.is_null() {
                    // Do not mark the zero page usable
                } else if frame >= buffer_phys && frame < buffer_phys_end {
                    // Do not mark "buffer" physical space usable
                } else {
                    unsafe {
                        allocator.unref(frame);
                    }
                }

                frame += PAGE_SIZE;
            }
        }
    }

    let stats = stats();
    const MEGA: usize = 1 * 1024 * 1024;
    info!(
        "Physical memory allocator initial stats: total={} ({}MB), free={} ({}MB)",
        stats.total,
        stats.total / MEGA,
        stats.free,
        stats.free / MEGA
    );
}

fn find_usable_region(memory_regions: &MemoryRegions, buffer_size: usize) -> PhysAddr {
    // Upper than 1M, and large enough to fit all memory
    // Merge usable regions
    const LOWER_BOUND: PhysAddr = PhysAddr::new(1 * 1024 * 1024);

    let mut start: PhysAddr = PhysAddr::zero();
    let mut size: usize = 0;

    for region in memory_regions.iter() {
        if region.kind != MemoryRegionKind::Usable {
            continue;
        }

        let region_start = PhysAddr::new(region.start);
        let region_size = (region.end - region.start) as usize;

        if region_start < LOWER_BOUND {
            continue;
        }

        if !start.is_null() && start + size == region_start {
            // Contigous: merge
            size += region_size;
            continue;
        }

        // Check if usable
        if size >= buffer_size {
            return start;
        }

        start = region_start;
        size = region_size;
    }

    // Check if usable
    if size >= buffer_size {
        return start;
    }

    panic!("Could not find suitable memory region for physical frame descriptors");
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub total: usize,
    pub free: usize,
}

pub fn stats() -> Stats {
    let allocator = ALLOCATOR.read();

    Stats {
        total: allocator.descriptors.len() * PAGE_SIZE,
        free: allocator.free_list.count * PAGE_SIZE,
    }
}

pub fn used(frame: PhysAddr) -> bool {
    let allocator = ALLOCATOR.read();
    debug_assert!(
        allocator.check_frame(frame),
        "Frame {:?} is not valid.",
        frame
    );

    unsafe {
        let desc = allocator.frame_to_desc(frame);
        (*desc).used()
    }
}

pub fn check_frame(frame: PhysAddr) -> bool {
    let allocator = ALLOCATOR.read();
    allocator.check_frame(frame)
}

pub fn allocate() -> Result<FrameRef, AllocatorError> {
    let mut allocator = ALLOCATOR.write();

    unsafe {
        let frame = allocator.allocate()?;
        Ok(FrameRef::new(frame))
    }
}

pub fn allocate_at(frame: PhysAddr) -> Result<FrameRef, AllocatorError> {
    let mut allocator = ALLOCATOR.write();

    unsafe {
        allocator.allocate_at(frame)?;
        Ok(FrameRef::new(frame))
    }
}

#[derive(Debug)]
pub struct FrameRef {
    /// Note: since we do not use the 0 frame, 0 is used as an "empty ref"
    frame: PhysAddr,
}

impl Clone for FrameRef {
    fn clone(&self) -> Self {
        if self.frame.is_null() {
            FrameRef::null()
        } else {
            let mut allocator = ALLOCATOR.write();

            unsafe {
                allocator.r#ref(self.frame);
                FrameRef::new(self.frame)
            }
        }
    }
}

impl Drop for FrameRef {
    fn drop(&mut self) {
        if !self.frame.is_null() {
            let mut allocator = ALLOCATOR.write();

            unsafe {
                allocator.unref(self.frame);
            }
        }
    }
}

impl FrameRef {
    /// Safety: no reference counting has been done, this only initialize an object with its frame.
    unsafe fn new(frame: PhysAddr) -> Self {
        Self { frame: frame }
    }

    /// Get a reference that points to nothing.
    pub fn null() -> Self {
        Self {
            frame: PhysAddr::zero(),
        }
    }

    /// Check if the reference points to nothing.
    pub fn is_null(&self) -> bool {
        return self.frame.is_null();
    }

    pub fn frame(&self) -> PhysAddr {
        self.frame
    }

    /// Use this function when you want to get back a frame previously borrowed with frame_ref.borrow()
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `unborrow()` call matchs `borrow()` call.
    pub unsafe fn unborrow(frame: PhysAddr) -> Self {
        debug_assert!(check_frame(frame));
        Self::new(frame)
    }

    /// Use this function when you want to borrow the frame, and forget the ref object (eg: it will be held by a page table, and we don't want to keep the FrameRef object alive with it)
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `borrow()` call match a `unborrow()` call.
    pub unsafe fn borrow(&mut self) -> PhysAddr {
        let frame = self.frame;
        self.frame = PhysAddr::zero();
        frame
    }
}
