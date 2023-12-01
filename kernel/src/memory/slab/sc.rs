//! A SCAllocator that can allocate fixed size objects.

use core::{
    alloc::Layout,
    ptr::{self, NonNull},
};

use log::trace;
use x86_64::VirtAddr;

use super::{
    page::{AllocablePage, Bitfield, ObjectPage, PageList, Rawlink},
    AllocationError, OBJECT_PAGE_METADATA_OVERHEAD,
};

/// A genius(?) const min()
///
/// # What this does
/// * create an array of the two elements you want to choose between
/// * create an arbitrary boolean expression
/// * cast said expresison to a usize
/// * use that value to index into the array created above
///
/// # Source
/// https://stackoverflow.com/questions/53619695/calculating-maximum-value-of-a-set-of-constant-expressions-at-compile-time
const fn cmin(a: usize, b: usize) -> usize {
    [a, b][(a > b) as usize]
}

/// A slab allocator allocates elements of a fixed size.
///
/// It maintains three internal lists of objects that implement `AllocablePage`
/// from which it can allocate memory.
///
///  * `empty_slabs`: Is a list of pages that the SCAllocator maintains, but
///    has 0 allocations in them, these can be given back to a requestor in case
///    of reclamation.
///  * `slabs`: A list of pages partially allocated and still have room for more.
///  * `full_slabs`: A list of pages that are completely allocated.
///
/// On allocation we allocate memory from `slabs`, however if the list is empty
/// we try to reclaim a page from `empty_slabs` before we return with an out-of-memory
/// error. If a page becomes full after the allocation we move it from `slabs` to
/// `full_slabs`.
///
/// Similarly, on dealloaction we might move a page from `full_slabs` to `slabs`
/// or from `slabs` to `empty_slabs` after we deallocated an object.
///
/// If an allocation returns `OutOfMemory` a client using SCAllocator can refill
/// it using the `refill` function.
pub struct SCAllocator<'a, P: AllocablePage = ObjectPage<'a>> {
    /// Maximum possible allocation size for this `SCAllocator`.
    size: usize,
    /// Keeps track of succeeded allocations.
    allocation_count: usize,
    /// max objects per page
    obj_per_page: usize,
    /// List of empty ObjectPages (nothing allocated in these).
    empty_slabs: PageList<'a, P>,
    /// List of partially used ObjectPage (some objects allocated but pages are not full).
    slabs: PageList<'a, P>,
    /// List of full ObjectPages (everything allocated in these don't need to search them).
    full_slabs: PageList<'a, P>,
}

/// Creates an instance of a scallocator, we do this in a macro because we
/// re-use the code in const and non-const functions
macro_rules! new_sc_allocator {
    ($size:expr) => {
        SCAllocator {
            size: $size,
            allocation_count: 0,
            obj_per_page: cmin((P::SIZE - OBJECT_PAGE_METADATA_OVERHEAD) / $size, 8 * 64),
            empty_slabs: PageList::new(),
            slabs: PageList::new(),
            full_slabs: PageList::new(),
        }
    };
}

impl<'a, P: AllocablePage> SCAllocator<'a, P> {
    const REBALANCE_COUNT: usize = 64;

    /// Create a new SCAllocator.
    pub const fn new(size: usize) -> SCAllocator<'a, P> {
        new_sc_allocator!(size)
    }

    /// Returns the maximum supported object size of this allocator.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Return the count of empty slabs currently in this allocator.
    pub fn empty_pages_count(&self) -> usize {
        self.empty_slabs.size()
    }

    /// Add a new ObjectPage.
    fn insert_partial_slab(&mut self, new_head: &'a mut P) {
        self.slabs.insert_front(new_head);
    }

    /// Add page to empty list.
    fn insert_empty(&mut self, new_head: &'a mut P) {
        assert_eq!(
            new_head as *const P as usize % P::SIZE,
            0,
            "Inserted page is not aligned to page-size."
        );
        self.empty_slabs.insert_front(new_head);
    }

    /// Since `dealloc` can not reassign pages without requiring a lock
    /// we check slabs and full slabs periodically as part of `alloc`
    /// and move them to the empty or partially allocated slab lists.
    fn check_page_assignments(&mut self) {
        for slab_page in self.full_slabs.iter_mut() {
            if !slab_page.is_full() {
                // We need to move it from self.full_slabs -> self.slabs
                trace!("move {:p} full -> partial", slab_page);
                self.move_full_to_partial(slab_page);
            }
        }

        for slab_page in self.slabs.iter_mut() {
            if slab_page.is_empty(self.obj_per_page) {
                // We need to move it from self.slabs -> self.empty_slabs
                trace!("move {:p} partial -> empty", slab_page);
                self.move_to_empty(slab_page);
            }
        }
    }

    /// Move a page from `slabs` to `empty_slabs`.
    fn move_to_empty(&mut self, page: &'a mut P) {
        let page_ptr = page as *const P;

        debug_assert!(self.slabs.contains(page_ptr));
        debug_assert!(
            !self.empty_slabs.contains(page_ptr),
            "Page {:p} already in emtpy_slabs",
            page_ptr
        );

        self.slabs.remove_from_list(page);
        self.empty_slabs.insert_front(page);

        debug_assert!(!self.slabs.contains(page_ptr));
        debug_assert!(self.empty_slabs.contains(page_ptr));
    }

    /// Move a page from `full_slabs` to `slab`.
    fn move_partial_to_full(&mut self, page: &'a mut P) {
        let page_ptr = page as *const P;

        debug_assert!(self.slabs.contains(page_ptr));
        debug_assert!(!self.full_slabs.contains(page_ptr));

        self.slabs.remove_from_list(page);
        self.full_slabs.insert_front(page);

        debug_assert!(!self.slabs.contains(page_ptr));
        debug_assert!(self.full_slabs.contains(page_ptr));
    }

    /// Move a page from `full_slabs` to `slab`.
    fn move_full_to_partial(&mut self, page: &'a mut P) {
        let page_ptr = page as *const P;

        debug_assert!(!self.slabs.contains(page_ptr));
        debug_assert!(self.full_slabs.contains(page_ptr));

        self.full_slabs.remove_from_list(page);
        self.slabs.insert_front(page);

        debug_assert!(self.slabs.contains(page_ptr));
        debug_assert!(!self.full_slabs.contains(page_ptr));
    }

    /// Tries to allocate a block of memory with respect to the `layout`.
    /// Searches within already allocated slab pages, if no suitable spot is found
    /// will try to use a page from the empty page list.
    ///
    /// # Arguments
    ///  * `sc_layout`: This is not the original layout but adjusted for the
    ///     SCAllocator size (>= original).
    fn try_allocate_from_pagelist(&mut self, sc_layout: Layout) -> *mut u8 {
        // TODO: Do we really need to check multiple slab pages (due to alignment)
        // If not we can get away with a singly-linked list and have 8 more bytes
        // for the bitfield in an ObjectPage.

        for slab_page in self.slabs.iter_mut() {
            let ptr = slab_page.allocate(sc_layout);
            if !ptr.is_null() {
                if slab_page.is_full() {
                    trace!("move {:p} partial -> full", slab_page);
                    self.move_partial_to_full(slab_page);
                }
                self.allocation_count += 1;
                return ptr;
            } else {
                continue;
            }
        }

        // Periodically rebalance page-lists (since dealloc can't do it for us)
        if self.allocation_count > SCAllocator::<P>::REBALANCE_COUNT {
            self.check_page_assignments();
            self.allocation_count = 0;
        }

        ptr::null_mut()
    }

    pub fn try_reclaim_pages<F>(&mut self, to_reclaim: usize, dealloc: &mut F) -> usize
    where
        F: FnMut(*mut P),
    {
        self.check_page_assignments();
        let mut reclaimed = 0;
        while reclaimed < to_reclaim {
            if let Some(page) = self.empty_slabs.pop() {
                dealloc(page as *mut P);
                reclaimed += 1;
            } else {
                break;
            }
        }

        reclaimed
    }

    /// Refill the SCAllocator
    ///
    /// # Safety
    /// ObjectPage needs to be empty etc.
    pub unsafe fn refill(&mut self, page: &'a mut P) {
        page.bitfield_mut()
            .initialize(self.size, P::SIZE - OBJECT_PAGE_METADATA_OVERHEAD);
        *page.prev() = Rawlink::none();
        *page.next() = Rawlink::none();
        trace!("adding page to SCAllocator {:p}", page);
        self.insert_empty(page);
    }

    /// Allocates a block of memory descriped by `layout`.
    ///
    /// Returns a pointer to a valid region of memory or an
    /// AllocationError.
    ///
    /// The function may also move around pages between lists
    /// (empty -> partial or partial -> full).
    pub fn allocate(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocationError> {
        trace!(
            "SCAllocator({}) is trying to allocate {:?}",
            self.size,
            layout
        );
        assert!(layout.size() <= self.size);
        assert!(self.size <= (P::SIZE - OBJECT_PAGE_METADATA_OVERHEAD));
        let new_layout = unsafe { Layout::from_size_align_unchecked(self.size, layout.align()) };
        assert!(new_layout.size() >= layout.size());

        let ptr = {
            // Try to allocate from partial slabs,
            // if we fail check if we have empty pages and allocate from there
            let ptr = self.try_allocate_from_pagelist(new_layout);
            if ptr.is_null() && self.empty_slabs.is_some() {
                // Re-try allocation in empty page
                let empty_page = self.empty_slabs.pop().expect("We checked head.is_some()");
                debug_assert!(!self.empty_slabs.contains(empty_page));

                let ptr = empty_page.allocate(layout);
                debug_assert!(!ptr.is_null(), "Allocation must have succeeded here.");

                trace!(
                    "move {:p} empty -> partial empty count {}",
                    empty_page,
                    self.empty_slabs.size()
                );
                // Move empty page to partial pages
                self.insert_partial_slab(empty_page);
                ptr
            } else {
                ptr
            }
        };

        let res = NonNull::new(ptr).ok_or(AllocationError::OutOfMemory);

        if !ptr.is_null() {
            trace!(
                "SCAllocator({}) allocated ptr=0x{:x}",
                self.size,
                ptr as usize
            );
        }

        res
    }

    /// Deallocates a previously allocated `ptr` described by `Layout`.
    ///
    /// May return an error in case an invalid `layout` is provided.
    /// The function may also move internal slab pages between lists partial -> empty
    /// or full -> partial lists.
    pub fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) -> Result<(), AllocationError> {
        assert!(layout.size() <= self.size);
        assert!(self.size <= (P::SIZE - OBJECT_PAGE_METADATA_OVERHEAD));
        trace!(
            "SCAllocator({}) is trying to deallocate ptr = {:p} layout={:?} P.size= {}",
            self.size,
            ptr,
            layout,
            P::SIZE
        );

        let page = VirtAddr::new(((ptr.as_ptr() as usize) & !(P::SIZE - 1)) as u64);

        // Figure out which page we are on and construct a reference to it
        // TODO: The linked list will have another &mut reference
        let slab_page: &mut P = unsafe { &mut *page.as_mut_ptr() };
        let new_layout = unsafe { Layout::from_size_align_unchecked(self.size, layout.align()) };

        let ret = slab_page.deallocate(ptr, new_layout);
        debug_assert!(ret.is_ok(), "Slab page deallocate won't fail at the moment");
        ret
    }
}
