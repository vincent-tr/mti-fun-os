use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, NonNull};
use log::debug;
use spin::Mutex;
use x86_64::{align_up, VirtAddr};

use crate::memory::PAGE_SIZE;

use super::kvm;
use super::slab::ZoneAllocator;

/// ALLOC is set as the system's default allocator, it's implementation follows below.
///
/// It's a ZoneAllocator wrapped inside a Mutex.
#[global_allocator]
static ALLOC: GlobalAllocator = GlobalAllocator(Mutex::new(ZoneAllocator::new()));

/// A GlobalAllocator that wraps the ZoneAllocator in a Mutex.
pub struct GlobalAllocator(Mutex<ZoneAllocator<'static>>);

impl GlobalAllocator {
    fn get_page_count(layout: Layout) -> usize {
        assert!(layout.align() <= PAGE_SIZE);
        return align_up(layout.size() as u64, PAGE_SIZE as u64) as usize;
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match layout.size() {
            0 => {
                panic!("Got zero-sized allocation request.");
            },
            1..=ZoneAllocator::MAX_ALLOC_SIZE => {
                debug!("Serving alloc request with slabs allocator {layout:?}");
                let mut zone_allocator = self.0.lock();
                match zone_allocator.allocate(layout) {
                    Ok(ptr) => ptr.as_ptr(),
                    Err(err) => {
                        // match all errors
                        match err {
                            super::slab::AllocationError::OutOfMemory => ptr::null_mut(),
                            super::slab::AllocationError::InvalidLayout => {
                                panic!("Invalid layout for slab allocation {layout:?}")
                            },
                        }
                    }
                }
            },
            _ => {
                let page_count = Self::get_page_count(layout);
                debug!("Serving alloc request with kvm allocator {layout:?} (page count = {page_count})");
                match kvm::allocate(page_count) {
                    Ok(addr) => addr.as_mut_ptr(),
                    Err(err) => {
                        // match all errors
                        match err {
                            kvm::AllocatorError::NoMemory => ptr::null_mut(),
                            kvm::AllocatorError::NoVirtualSpace => ptr::null_mut(),
                        }
                    }
                }
            },
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!(!ptr.is_null());

        match layout.size() {
            0 => {
                panic!("Got zero-sized deallocation request.");
            },
            1..=ZoneAllocator::MAX_ALLOC_SIZE => {
                debug!("Serving dealloc request with slabs allocator {layout:?}");
                let mut zone_allocator = self.0.lock();
                zone_allocator.deallocate(NonNull::new_unchecked(ptr), layout);
            },
            _ => {
                let page_count = Self::get_page_count(layout);
                debug!("Serving dealloc request with kvm allocator {layout:?} (page count = {page_count})");
                kvm::deallocate(VirtAddr::from_ptr(ptr), page_count);
            },
        }
    }
}
