use core::{
    cmp::Ordering,
    mem::size_of,
    ptr::{null_mut, slice_from_raw_parts_mut},
};

use bootloader_api::info::{MemoryRegions, MemoryRegionKind};
use spin::RwLock;
use x86_64::{PhysAddr, VirtAddr};

pub const PAGE_SIZE: u64 = 4096;

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
            prev: null_mut(),
            next: null_mut(),
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
}

struct List {
    head: *mut Descriptor,
    count: usize,
}

impl List {
    const fn new() -> Self {
        Self {
            head: null_mut(),
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
            self.head == null_mut();
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

        (*desc).prev = null_mut();
        (*desc).next = null_mut();
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
            descriptors: slice_from_raw_parts_mut(null_mut(), 0),
            used_list: List::new(),
            free_list: List::new(),
        }
    }

    pub const fn needed_buffer_size(page_count: usize) -> usize {
        page_count * size_of::<Descriptor>()
    }

    unsafe fn init_descriptors(&mut self, buffer: VirtAddr, buffer_size: usize) {
        debug_assert!(buffer.is_aligned(PAGE_SIZE));
        debug_assert!((buffer + buffer_size).is_aligned(PAGE_SIZE));

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

    unsafe fn r#ref(&mut self, frame: PhysAddr) {
        let desc = self.frame_to_desc(frame);
        let desc_ref = &mut (*desc);

        desc_ref.r#ref();
    }

    /// Returns true if the page has still references, false if it has been deallocated
    unsafe fn unref(&mut self, frame: PhysAddr) -> bool {
        let desc = self.frame_to_desc(frame);
        let desc_ref = &mut (*desc);

        let has_ref = desc_ref.unref();

        if !has_ref {
            self.used_list.remove(desc);
            self.free_list.add(desc);
        }

        has_ref
    }

    unsafe fn frame_to_desc(&self, frame: PhysAddr) -> *mut Descriptor {
        let index = (frame.as_u64() / PAGE_SIZE) as usize;
        self.descriptors.as_mut_ptr().add(index)
    }

    unsafe fn desc_to_frame(&self, desc: *mut Descriptor) -> PhysAddr {
        let index = desc.sub_ptr(self.descriptors.as_mut_ptr());
        PhysAddr::new(index as u64 * PAGE_SIZE)
    }
}

pub enum AllocatorError {
    NoMemory,
}

static ALLOCATOR: RwLock<Allocator> = RwLock::new(Allocator::new());

pub fn init(phys_mapping: VirtAddr, memory_regions: &'static MemoryRegions) {
    // memory regions looks ordered.
    debug_assert!(memory_regions.is_sorted_by_key(|region| { region.start }));

    let end = memory_regions.last().unwrap().end;
    debug_assert!(PhysAddr::new(end).is_aligned(PAGE_SIZE));

    let count = (end / PAGE_SIZE) as usize;
    let buffer_size = Allocator::needed_buffer_size(count);

    // TODO: map an area of this size somehow
    let buffer: VirtAddr = VirtAddr::zero();

    let mut allocator = ALLOCATOR.write();
    unsafe {
        allocator.init_descriptors(buffer, buffer_size);
    }

    // Note: by default all pages are created as reserved

    for memory_region in memory_regions.iter() {
        if memory_region.kind == MemoryRegionKind::Usable {
            let mut frame = PhysAddr::new(memory_region.start);
            while frame < PhysAddr::new(memory_region.end) {
                // Do not mark the zero page usable
                if frame.is_null() {
                    continue;
                }

                unsafe {
                    allocator.unref(frame);
                }

                frame += PAGE_SIZE;
            }
        }
    }

    // TODO: mark "buffer" physical space reserved

}

pub fn allocate() -> Result<FrameRef, AllocatorError> {
    let mut allocator = ALLOCATOR.write();

    unsafe {
        let frame = allocator.allocate()?;
        Ok(FrameRef::new(frame))
    }
}

#[derive(Debug)]
struct FrameRef {
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

    /// Use this function when you want to get back a frame previously borrowed with frame_ref.borrow()
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `unborrow()` call matchs `borrow()` call.
    pub unsafe fn unborrow(frame: PhysAddr) -> Self {
        Self::new(frame)
    }

    /// Use this function when you want to borrow the frame, and forget the ref object (eg: it will be held by a page table, and we don't want to keep the FrameRef object alive with it)
    ///
    /// # Safety
    ///
    /// This function is unsafe because there is no way to check that `borrow()` call match a `unborrow()` call.
    pub unsafe fn borrow(&mut self) {
        self.frame = PhysAddr::zero();
    }
}
