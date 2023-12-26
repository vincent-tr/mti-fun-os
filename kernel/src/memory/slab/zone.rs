//! A ZoneAllocator to allocate arbitrary object sizes (up to `ZoneAllocator::MAX_ALLOC_SIZE`)
//!
//! The ZoneAllocator achieves this by having many `SCAllocator`

use core::{alloc::Layout, panic, ptr::NonNull};

use log::trace;
use x86_64::VirtAddr;

use crate::memory::kvm;

use super::{AllocationError, ObjectPage, SCAllocator};

/// A zone allocator for arbitrary sized allocations.
///
/// Has a bunch of `SCAllocator` and through that can serve allocation
/// requests for many different object sizes up to (MAX_SIZE_CLASSES) by selecting
/// the right `SCAllocator` for allocation and deallocation.
///
/// The allocator provides to refill functions `refill` and `refill_large`
/// to provide the underlying `SCAllocator` with more memory in case it runs out.
pub struct ZoneAllocator<'a> {
    slabs: [SCAllocator<'a, ObjectPage<'a>>; ZoneAllocator::MAX_CLASSES],
}

impl<'a> ZoneAllocator<'a> {
    /// Maximum size that allocated (1024)
    pub const MAX_ALLOC_SIZE: usize = 1 << 10;

    /// How many slabs we have.
    const MAX_CLASSES: usize = 8;

    pub const fn new() -> ZoneAllocator<'a> {
        ZoneAllocator {
            slabs: [
                SCAllocator::new(1 << 3),  // 8
                SCAllocator::new(1 << 4),  // 16
                SCAllocator::new(1 << 5),  // 32
                SCAllocator::new(1 << 6),  // 64
                SCAllocator::new(1 << 7),  // 128
                SCAllocator::new(1 << 8),  // 256
                SCAllocator::new(1 << 9),  // 512
                SCAllocator::new(1 << 10), // 1024
            ],
        }
    }

    /// Figure out index into zone array to get the correct slab allocator for that size.
    fn get_slab_index(requested_size: usize) -> Option<usize> {
        match requested_size {
            0..=8 => Some(0),
            9..=16 => Some(1),
            17..=32 => Some(2),
            33..=64 => Some(3),
            65..=128 => Some(4),
            129..=256 => Some(5),
            257..=512 => Some(6),
            513..=1024 => Some(7),
            _ => None,
        }
    }

    pub fn get_allocation_size(&self, layout: Layout) -> Option<usize> {
        let res = ZoneAllocator::get_slab_index(layout.size());
        if res.is_none() {
            return None;
        }
        let index = res.unwrap();
        let slab = &self.slabs[index];
        Some(slab.size())
    }

    /// Allocate a pointer to a block of memory described by `layout`.
    pub fn allocate(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocationError> {
        let res = ZoneAllocator::get_slab_index(layout.size());
        if res.is_none() {
            return Err(AllocationError::InvalidLayout);
        }
        let index = res.unwrap();

        let slab = &mut self.slabs[index];

        match slab.allocate(layout) {
            Ok(buf) => Ok(buf),
            Err(AllocationError::OutOfMemory) => {
                // refill and re-try
                ZoneAllocator::refill(slab)?;
                slab.allocate(layout)
            }
            Err(err) => Err(err),
        }
    }

    /// Deallocates a pointer to a block of memory, which was
    /// previously allocated by `allocate`.
    ///
    /// # Arguments
    ///  * `ptr` - Address of the memory location to free.
    ///  * `layout` - Memory layout of the block pointed to by `ptr`.
    pub fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let index = ZoneAllocator::get_slab_index(layout.size()).expect("Invalid layout.");
        let slab = &mut self.slabs[index];

        slab.deallocate(ptr, layout);

        let to_reclaim = slab.empty_pages_count();
        match to_reclaim {
            0 => {}
            1 => {
                trace!("Reclaim 1 page to slab allocator {}", slab.size());

                let mut dealloc = |ptr: *mut _| {
                    kvm::deallocate(VirtAddr::from_ptr(ptr), 1);
                };

                let reclaimed = slab.try_reclaim_pages(1, &mut dealloc);

                assert!(reclaimed == 1);
            }

            many => {
                panic!("Unexpected to_reclaim={}", many);
            }
        };
    }

    fn refill(slab: &mut SCAllocator) -> Result<(), AllocationError> {
        trace!("Refill 1 page to slab allocator {}", slab.size());

        match kvm::allocate(1) {
            Ok(addr) => {
                unsafe {
                    slab.refill(&mut *addr.as_mut_ptr());
                }

                Ok(())
            }
            Err(err) => {
                // ensure we matched all values
                match err {
                    kvm::AllocatorError::NoMemory => Err(AllocationError::OutOfMemory),
                    kvm::AllocatorError::NoVirtualSpace => Err(AllocationError::OutOfMemory),
                }
            }
        }
    }
}
