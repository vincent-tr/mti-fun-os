use core::{alloc::Layout, mem, ptr::NonNull, usize};

use spin::RwLock;
use x86_64::{structures::paging::mapper::MapToError, VirtAddr};

use super::{
    buddy::{self, BuddyAllocator},
    paging::{self, Permissions},
    phys,
    slab::{self, SCAllocator},
    PAGE_SIZE, VMALLOC_END, VMALLOC_START,
};

/*

4096 = 1 >> 12
4096 >> 17 = 512G

=> Buddy allocator for vm space
=> reserve 1G for kernel

*/

const BUDDY_ORDERS: usize = 16;

#[derive(Debug)]
pub enum AllocatorError {
    NoMemory,
    NoVirtualSpace,
}

pub struct Allocator<'a> {
    buddy_allocator: BuddyAllocator<BUDDY_ORDERS, NodeAllocator<'a>>,
}

impl<'a> Allocator<'a> {
    pub const fn new() -> Self {
        Self {
            buddy_allocator: BuddyAllocator::new(NodeAllocator::new(), PAGE_SIZE),
        }
    }

    pub fn allocate(&mut self, page_count: usize) -> Result<VirtAddr, AllocatorError> {
        assert!(page_count > 0);
        let layout = Self::build_layout(page_count);

        let alloc_res = self.buddy_allocator.alloc(layout);
        if let Err(err) = alloc_res {
            // Ensure we matched all types
            match err {
                buddy::AllocatorError::NoMemory => {}
            }

            return Err(AllocatorError::NoVirtualSpace);
        }

        let addr = alloc_res.unwrap();

        match self.map_phys_alloc(addr, page_count) {
            Ok(_) => {
                self.buddy_allocator
                    .node_allocator()
                    .check_reservation(self);
                Ok(addr)
            }
            Err(err) => {
                // remove address space reservation
                self.buddy_allocator.dealloc(addr, layout);

                Err(err)
            }
        }
    }

    pub fn deallocate(&mut self, addr: VirtAddr, page_count: usize) {
        assert!(page_count > 0);
        assert!(addr.is_aligned(PAGE_SIZE as u64));
        assert!(addr >= VMALLOC_START);
        assert!(addr < VMALLOC_END);

        self.unmap_phys_alloc(addr, page_count);

        self.buddy_allocator
            .dealloc(addr, Self::build_layout(page_count));

        self.buddy_allocator
            .node_allocator()
            .check_reservation(self);
    }

    // Note: if part of the allocation fails, it is properly removed
    fn map_phys_alloc(&mut self, addr: VirtAddr, page_count: usize) -> Result<(), AllocatorError> {
        for page_index in 0..page_count {
            let frame_res = phys::allocate();

            if let Err(err) = frame_res {
                // Ensure we matched all types
                match err {
                    phys::AllocatorError::NoMemory => {}
                }

                // Remove pages allocated so far
                if page_index > 0 {
                    self.unmap_phys_alloc(addr, page_index);
                }

                return Err(AllocatorError::NoMemory);
            }

            let page_addr = addr + page_index * PAGE_SIZE;
            let mut frame = frame_res.unwrap();
            let perms = Permissions::READ | Permissions::WRITE;

            if let Err(err) =
                unsafe { paging::KERNEL_ADDRESS_SPACE.map(page_addr, &mut frame, perms) }
            {
                // Remove pages allocated so far
                if page_index > 0 {
                    self.unmap_phys_alloc(addr, page_index);
                }

                match err {
                    MapToError::FrameAllocationFailed => return Err(AllocatorError::NoMemory),
                    MapToError::ParentEntryHugePage => {
                        panic!("Unexpected map error: ParentEntryHugePage")
                    }
                    MapToError::PageAlreadyMapped(_) => {
                        panic!("Unexpected map error: PageAlreadyMapped")
                    }
                }
            }
        }

        Ok(())
    }

    fn unmap_phys_alloc(&mut self, addr: VirtAddr, page_count: usize) {
        for page_index in 0..page_count {
            let page_addr = addr + page_index * PAGE_SIZE;
            let frame = unsafe {
                paging::KERNEL_ADDRESS_SPACE
                    .unmap(page_addr)
                    .expect("could not unmap page")
            };
            mem::drop(frame);
        }
    }

    fn build_layout(page_count: usize) -> Layout {
        unsafe { Layout::from_size_align_unchecked(page_count * PAGE_SIZE, PAGE_SIZE) }
    }
}

static ALLOCATOR: RwLock<Allocator> = RwLock::new(Allocator::new());

pub fn init() {
    let mut allocator = ALLOCATOR.write();

    // Bootstrap reservation:
    // Manually reserve one page
    // Note: the page address must match the first place where the buddy allocator will allocate
    let initial_page = VMALLOC_START;

    allocator
        .map_phys_alloc(initial_page, 1)
        .expect("Allocator initialization failed. (physical frame mapping)");
    allocator.buddy_allocator.node_allocator().init(initial_page);

    // Now we can init 
    allocator.buddy_allocator.set_area(VMALLOC_START, VMALLOC_END);

    // Mark `initial_page` as used
    let layout = Allocator::<'static>::build_layout(1);
    let addr = allocator.buddy_allocator.alloc(layout).expect("Allocator initialization failed. (buddy allocator)");

    // Ensure address was right
    assert!(initial_page == addr, "Allocator initialization failed. (initial page does not match first allocation)");

    allocator.buddy_allocator.node_allocator().check_reservation(&mut allocator);
}

pub fn allocate(page_count: usize) -> Result<VirtAddr, AllocatorError> {
    let mut allocator = ALLOCATOR.write();

    allocator.allocate(page_count)
}

pub fn deallocate(addr: VirtAddr, page_count: usize) {
    let mut allocator = ALLOCATOR.write();

    allocator.deallocate(addr, page_count)
}

/// Provide node allocator to buddy allocator
///
/// Uses a slab allocator, with an empty slab as reservation to avoid endless recursive allocation
///
struct NodeAllocator<'a> {
    allocator: SCAllocator<'a>,
}

impl<'a> NodeAllocator<'a> {
    pub const fn new() -> Self {
        let layout = Layout::new::<buddy::ListNode>();
        Self {
            allocator: SCAllocator::new(layout.size()),
        }
    }

    pub fn check_reservation(&mut self, main_allocator: &mut Allocator) {
        // A page can hold
        //  (PAGE_SIZE - OBJECT_PAGE_METADATA_OVERHEAD) / sizeof(ListNode)
        //  (4096 - 80) / 16 = 251
        // we ensure that we always have an empty page.
        match self.allocator.empty_pages_count() {
            0 => {
                let addr = main_allocator
                    .allocate(1)
                    .expect("Cannot fail allocation for reservation");
                unsafe {
                    self.allocator.refill(&mut *addr.as_mut_ptr());
                }
            }
            1 => {
                // Perfect.
            }
            count => {
                let mut dealloc = |ptr: *mut _| {
                    main_allocator.deallocate(VirtAddr::from_ptr(ptr), 1);
                };

                let reclaimed = self.allocator.try_reclaim_pages(count - 1, &mut dealloc);
                assert!(reclaimed == count - 1);
            }
        }
    }

    pub fn init(&mut self, initial_page: VirtAddr) {
        unsafe {
            self.allocator.refill(&mut *initial_page.as_mut_ptr());
        }
    }
}

impl<'a> buddy::NodeAllocator for NodeAllocator<'a> {
    unsafe fn allocate(&mut self) -> *mut buddy::ListNode {
        let layout = Layout::new::<buddy::ListNode>();

        match self.allocator.allocate(layout) {
            Ok(ptr) => ptr.as_ptr() as *mut buddy::ListNode,
            Err(slab::AllocationError::OutOfMemory) => {
                panic!("Allocation failed. This should not happen due to reservations");
            }
            Err(slab::AllocationError::InvalidLayout) => {
                panic!("InvalidLayout");
            }
        }
    }

    unsafe fn deallocate(&mut self, node: *mut buddy::ListNode) {
        let layout = Layout::new::<buddy::ListNode>();

        self.allocator
            .deallocate(NonNull::new_unchecked(node as *mut u8), layout);
    }
}
