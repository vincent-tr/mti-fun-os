use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};
use log::trace;
use spin::Mutex;
use x86_64::{align_up, VirtAddr};

use crate::memory::PAGE_SIZE;

use super::slab::ZoneAllocator;
use super::{kvm, KallocStats};

#[global_allocator]
pub static ALLOC: GlobalAllocator = GlobalAllocator::new();

/// A GlobalAllocator that wraps the ZoneAllocator in a Mutex.
pub struct GlobalAllocator {
    slabs_allocator: Mutex<ZoneAllocator<'static>>,

    slabs_user: AtomicUsize,
    slabs_allocated: AtomicUsize,
    kvm_user: AtomicUsize,
    kvm_allocated: AtomicUsize,
}

impl GlobalAllocator {
    pub const fn new() -> Self {
        Self {
            slabs_allocator: Mutex::new(ZoneAllocator::new()),
            slabs_user: AtomicUsize::new(0),
            slabs_allocated: AtomicUsize::new(0),
            kvm_user: AtomicUsize::new(0),
            kvm_allocated: AtomicUsize::new(0),
        }
    }

    fn get_page_count(layout: Layout) -> usize {
        assert!(layout.align() <= PAGE_SIZE);
        return align_up(layout.size() as u64, PAGE_SIZE as u64) as usize;
    }

    pub fn stats(&self) -> KallocStats {
        // Note: may be inconsistent
        KallocStats {
            slabs_user: self.slabs_user.load(Ordering::Relaxed),
            slabs_allocated: self.slabs_allocated.load(Ordering::Relaxed),
            kvm_user: self.kvm_user.load(Ordering::Relaxed),
            kvm_allocated: self.kvm_allocated.load(Ordering::Relaxed),
        }
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match layout.size() {
            0 => {
                panic!("Got zero-sized allocation request.");
            }
            1..=ZoneAllocator::MAX_ALLOC_SIZE => {
                trace!("Serving alloc request with slabs allocator {layout:?}");
                let mut slabs_allocator = self.slabs_allocator.lock();
                match slabs_allocator.allocate(layout) {
                    Ok(ptr) => {
                        self.slabs_user.fetch_add(layout.size(), Ordering::Relaxed);
                        self.slabs_allocated.fetch_add(
                            slabs_allocator
                                .get_allocation_size(layout)
                                .expect("Unexpected layout error."),
                            Ordering::Relaxed,
                        );

                        ptr.as_ptr()
                    }
                    Err(err) => {
                        // match all errors
                        match err {
                            super::slab::AllocationError::OutOfMemory => ptr::null_mut(),
                            super::slab::AllocationError::InvalidLayout => {
                                panic!("Invalid layout for slab allocation {layout:?}")
                            }
                        }
                    }
                }
            }
            _ => {
                let page_count = Self::get_page_count(layout);
                trace!("Serving alloc request with kvm allocator {layout:?} (page count = {page_count})");
                match kvm::allocate(page_count) {
                    Ok(addr) => {
                        self.kvm_user.fetch_add(layout.size(), Ordering::Relaxed);
                        self.kvm_allocated
                            .fetch_add(page_count * PAGE_SIZE, Ordering::Relaxed);

                        addr.as_mut_ptr()
                    }
                    Err(err) => {
                        // match all errors
                        match err {
                            kvm::AllocatorError::NoMemory => ptr::null_mut(),
                            kvm::AllocatorError::NoVirtualSpace => ptr::null_mut(),
                        }
                    }
                }
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!(!ptr.is_null());

        match layout.size() {
            0 => {
                panic!("Got zero-sized deallocation request.");
            }
            1..=ZoneAllocator::MAX_ALLOC_SIZE => {
                trace!("Serving dealloc request with slabs allocator {layout:?}");
                let mut slabs_allocator = self.slabs_allocator.lock();
                slabs_allocator.deallocate(NonNull::new_unchecked(ptr), layout);

                self.slabs_user.fetch_sub(layout.size(), Ordering::Relaxed);
                self.slabs_allocated.fetch_sub(
                    slabs_allocator
                        .get_allocation_size(layout)
                        .expect("Unexpected layout error."),
                    Ordering::Relaxed,
                );
            }
            _ => {
                let page_count = Self::get_page_count(layout);
                trace!("Serving dealloc request with kvm allocator {layout:?} (page count = {page_count})");
                kvm::deallocate(VirtAddr::from_ptr(ptr), page_count);

                self.kvm_user.fetch_sub(layout.size(), Ordering::Relaxed);
                self.kvm_allocated
                    .fetch_sub(page_count * PAGE_SIZE, Ordering::Relaxed);
            }
        }
    }
}
