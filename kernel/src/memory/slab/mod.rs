// from https://github.com/gz/rust-slabmalloc

use core::{ptr::NonNull, alloc::Layout};

use self::page::ObjectPage;

mod page;
mod sc;
mod zone;

pub use sc::SCAllocator;
pub use zone::ZoneAllocator;

/// How many bytes in the page are used by allocator meta-data.
pub const OBJECT_PAGE_METADATA_OVERHEAD: usize = 80;

/// Error that can be returned for `allocation` and `deallocation` requests.
#[derive(Debug)]
pub enum AllocationError {
    /// Can't satisfy the allocation request for Layout because the allocator
    /// does not have enough memory (you may be able to `refill` it).
    OutOfMemory,
    /// Allocator can't deal with the provided size of the Layout.
    InvalidLayout,
}

/// Allocator trait to be implemented by users of slabmalloc to provide memory to slabmalloc.
///
/// # Safety
/// Needs to adhere to safety requirements of a rust allocator (see GlobalAlloc et. al.).
pub unsafe trait Allocator<'a> {
    fn allocate(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocationError>;
    fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) -> Result<(), AllocationError>;

    /// Refill the allocator with a [`ObjectPage`].
    ///
    /// # Safety
    /// TBD (this API needs to change anyways, likely new page should be a raw pointer)
    unsafe fn refill(
        &mut self,
        layout: Layout,
        new_page: &'a mut ObjectPage<'a>,
    ) -> Result<(), AllocationError>;
}