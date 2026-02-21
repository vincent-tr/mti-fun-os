use core::{alloc::Layout, mem, ptr::NonNull, usize};

use log::{debug, info};
use spin::RwLock;
use x86_64::{PhysAddr, VirtAddr, structures::paging::mapper::MapToError};

use super::{
    AdditionalFlags, FrameRef, KvmStats,
    buddy::{self, BuddyAllocator},
    config::{KERNEL_START, PAGE_SIZE, VMALLOC_END, VMALLOC_START},
    paging::{self, Permissions},
    phys,
    slab::{self, SCAllocator},
};

/*

1 >> 12 = 4096
1 >> 39 = 512G (level 4 size)
512G = 0x8000000000

4096 >> 18 =

=> Buddy allocator for vm space
=> reserve 1G for kernel

*/

// the whole Level 4 entry, to get the right buddy shape.
// kernel space will be removed after.
const KERNEL_SPACE_SIZE: u64 = VMALLOC_END.as_u64() - KERNEL_START.as_u64();

const BUDDY_ORDERS: usize =
    KERNEL_SPACE_SIZE.trailing_zeros() as usize - PAGE_SIZE.trailing_zeros() as usize;

#[derive(Debug)]
pub enum AllocatorError {
    NoMemory,
    NoVirtualSpace,
}

struct Allocator<'a> {
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

        let addr = self.reserve(page_count)?;

        match self.map_phys_alloc(addr, page_count) {
            Ok(_) => Ok(addr),
            Err(err) => {
                // remove address space reservation
                self.unreserve(addr, page_count);

                Err(err)
            }
        }
    }

    pub fn allocate_with_frames(
        &mut self,
        frames: &mut [FrameRef],
    ) -> Result<VirtAddr, AllocatorError> {
        assert!(frames.len() > 0);

        let addr = self.reserve(frames.len())?;

        match self.map_phys_frames(addr, frames) {
            Ok(_) => Ok(addr),
            Err(err) => {
                // remove address space reservation
                self.unreserve(addr, frames.len());

                Err(err)
            }
        }
    }

    pub fn allocate_iomem(
        &mut self,
        phys_addr: PhysAddr,
        page_count: usize,
        perms: Permissions,
    ) -> Result<VirtAddr, AllocatorError> {
        assert!(page_count > 0);
        assert!(phys_addr.is_aligned(PAGE_SIZE as u64));
        // Only accept read or write (or both)
        assert!(
            perms.intersects(Permissions::READ | Permissions::WRITE)
                && !perms.intersects(Permissions::EXECUTE)
        );

        let addr = self.reserve(page_count)?;

        match self.map_iomem(addr, phys_addr, page_count, perms) {
            Ok(_) => Ok(addr),
            Err(err) => {
                // remove address space reservation
                self.unreserve(addr, page_count);

                Err(err)
            }
        }
    }

    pub fn deallocate(&mut self, addr: VirtAddr, page_count: usize) {
        assert!(page_count > 0);
        assert!(addr.is_aligned(PAGE_SIZE as u64));
        assert!(addr >= VMALLOC_START);
        assert!(addr < VMALLOC_END);

        self.unmap_phys(addr, page_count);

        self.unreserve(addr, page_count);
    }

    pub fn deallocate_iomem(&mut self, addr: VirtAddr, page_count: usize) {
        assert!(page_count > 0);
        assert!(addr.is_aligned(PAGE_SIZE as u64));
        assert!(addr >= VMALLOC_START);
        assert!(addr < VMALLOC_END);

        self.unmap_iomem(addr, page_count);

        self.unreserve(addr, page_count);
    }

    fn reserve(&mut self, page_count: usize) -> Result<VirtAddr, AllocatorError> {
        let layout = Self::build_layout(page_count);

        match self.buddy_allocator.alloc(layout) {
            Ok(addr) => Ok(addr),
            Err(err) => {
                // Ensure we matched all types
                match err {
                    buddy::AllocatorError::NoMemory => {}
                }

                return Err(AllocatorError::NoVirtualSpace);
            }
        }
    }

    fn unreserve(&mut self, addr: VirtAddr, page_count: usize) {
        self.buddy_allocator
            .dealloc(addr, Self::build_layout(page_count));
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
                    self.unmap_phys(addr, page_index);
                }

                return Err(AllocatorError::NoMemory);
            }

            let page_addr = addr + ((page_index * PAGE_SIZE) as u64);
            let mut frame = frame_res.unwrap();
            let perms = Permissions::READ | Permissions::WRITE;

            if let Err(err) = unsafe {
                paging::get_kernel_address_space().map(page_addr, frame.frame(), perms, None)
            } {
                // Remove pages allocated so far
                if page_index > 0 {
                    self.unmap_phys(addr, page_index);
                }

                match err {
                    MapToError::FrameAllocationFailed => return Err(AllocatorError::NoMemory),
                    MapToError::ParentEntryHugePage => {
                        panic!("Unexpected map error: ParentEntryHugePage {page_addr:?}")
                    }
                    MapToError::PageAlreadyMapped(_) => {
                        panic!("Unexpected map error: PageAlreadyMapped {page_addr:?}")
                    }
                }
            }

            // Mark the mapped frame as borrowed
            unsafe { frame.borrow() };
        }

        Ok(())
    }

    fn map_phys_frames(
        &mut self,
        addr: VirtAddr,
        frames: &mut [FrameRef],
    ) -> Result<(), AllocatorError> {
        for (page_index, frame) in frames.iter_mut().enumerate() {
            let page_addr = addr + ((page_index * PAGE_SIZE) as u64);
            let perms = Permissions::READ | Permissions::WRITE;

            if let Err(err) = unsafe {
                paging::get_kernel_address_space().map(page_addr, frame.frame(), perms, None)
            } {
                // Remove pages allocated so far
                if page_index > 0 {
                    self.unmap_phys(addr, page_index);
                }

                match err {
                    MapToError::FrameAllocationFailed => return Err(AllocatorError::NoMemory),
                    MapToError::ParentEntryHugePage => {
                        panic!("Unexpected map error: ParentEntryHugePage {page_addr:?}")
                    }
                    MapToError::PageAlreadyMapped(_) => {
                        panic!("Unexpected map error: PageAlreadyMapped {page_addr:?}")
                    }
                }
            }

            // Mark the mapped frame as borrowed
            unsafe { frame.borrow() };
        }

        Ok(())
    }
    fn map_iomem(
        &mut self,
        addr: VirtAddr,
        phys_addr: PhysAddr,
        page_count: usize,
        perms: Permissions,
    ) -> Result<(), AllocatorError> {
        for page_index in 0..page_count {
            let page_addr = addr + ((page_index * PAGE_SIZE) as u64);
            let frame_addr = phys_addr + ((page_index * PAGE_SIZE) as u64);

            // Strongly uncacheable
            let mut iomem_flags = AdditionalFlags::new();
            iomem_flags.write_through(true);
            iomem_flags.no_cache(true);

            if let Err(err) = unsafe {
                paging::get_kernel_address_space().map(
                    page_addr,
                    frame_addr,
                    perms,
                    Some(iomem_flags),
                )
            } {
                // Remove pages allocated so far
                if page_index > 0 {
                    self.unmap_iomem(addr, page_index);
                }

                match err {
                    MapToError::FrameAllocationFailed => return Err(AllocatorError::NoMemory),
                    MapToError::ParentEntryHugePage => {
                        panic!("Unexpected map error: ParentEntryHugePage {page_addr:?}")
                    }
                    MapToError::PageAlreadyMapped(_) => {
                        panic!("Unexpected map error: PageAlreadyMapped {page_addr:?}")
                    }
                }
            }
        }

        Ok(())
    }

    fn unmap_phys(&mut self, addr: VirtAddr, page_count: usize) {
        for page_index in 0..page_count {
            let page_addr = addr + ((page_index * PAGE_SIZE) as u64);
            let phys_addr = unsafe {
                paging::get_kernel_address_space()
                    .unmap(page_addr)
                    .expect("could not unmap page")
            };

            // Unborrowed and drop the frame
            let frame = unsafe { FrameRef::unborrow(phys_addr) };
            mem::drop(frame);
        }
    }

    fn unmap_iomem(&mut self, addr: VirtAddr, page_count: usize) {
        for page_index in 0..page_count {
            let page_addr = addr + ((page_index * PAGE_SIZE) as u64);
            unsafe {
                paging::get_kernel_address_space()
                    .unmap(page_addr)
                    .expect("could not unmap page")
            };
        }
    }

    fn build_layout(page_count: usize) -> Layout {
        unsafe { Layout::from_size_align_unchecked(page_count * PAGE_SIZE, PAGE_SIZE) }
    }

    fn check_slab_reservations(&mut self) {
        // We want to write:
        //
        // self.buddy_allocator
        //     .node_allocator()
        //     .check_reservation(self);
        //
        // But this break borrow checker rules.
        // Unless better is found, let's break borrowing with raw pointers..
        let node_allocator_ptr: *mut NodeAllocator = self.buddy_allocator.node_allocator();
        let node_allocator = unsafe { &mut *node_allocator_ptr };
        node_allocator.check_reservation(self);
    }
}

static ALLOCATOR: RwLock<Allocator> = RwLock::new(Allocator::new());

pub fn init() {
    let mut allocator = ALLOCATOR.write();

    // Bootstrap reservation:
    // Manually reserve one page
    // Note: the page address must match the first place where the buddy allocator will allocate
    let initial_page = VMALLOC_START;
    assert!(initial_page.is_aligned(PAGE_SIZE as u64));

    allocator
        .map_phys_alloc(initial_page, 1)
        .expect("Allocator initialization failed. (physical frame mapping)");
    allocator
        .buddy_allocator
        .node_allocator()
        .init(initial_page);

    // Now we can init
    allocator
        .buddy_allocator
        .set_area(VMALLOC_START, VMALLOC_END);

    // Mark `initial_page` as used
    let layout = Allocator::<'static>::build_layout(1);
    let addr = allocator
        .buddy_allocator
        .alloc(layout)
        .expect("Allocator initialization failed. (buddy allocator)");

    // Ensure address was right
    assert!(
        initial_page == addr,
        "Allocator initialization failed. (initial page does not match first allocation)"
    );

    allocator.check_slab_reservations();

    let obj_per_slab = allocator
        .buddy_allocator
        .node_allocator()
        .allocator
        .obj_per_slab();

    mem::drop(allocator);

    info!(
        "Initialized KVM. (Buddy orders = {}, Nodes per slab = {}, VM Start={:?}, VM End={:?})",
        BUDDY_ORDERS, obj_per_slab, VMALLOC_START, VMALLOC_END
    );
}

pub fn stats() -> KvmStats {
    let allocator = ALLOCATOR.read();

    let buddy_stats = allocator.buddy_allocator.stats();

    KvmStats {
        used: buddy_stats.allocated,
        total: buddy_stats.total,
    }
}

pub fn allocate(page_count: usize) -> Result<VirtAddr, AllocatorError> {
    let mut allocator = ALLOCATOR.write();

    let res = allocator.allocate(page_count);

    if res.is_ok() {
        allocator.check_slab_reservations();
    }

    res
}

/// Map only some already-allocated phys frames into kvm space
///
/// ### Note
/// The frames are cloned then borrowed.
pub fn allocate_with_frames(frames: &mut [FrameRef]) -> Result<VirtAddr, AllocatorError> {
    let mut allocator = ALLOCATOR.write();

    let res = allocator.allocate_with_frames(frames);

    if res.is_ok() {
        allocator.check_slab_reservations();
    }

    res
}
pub fn deallocate(addr: VirtAddr, page_count: usize) {
    let mut allocator = ALLOCATOR.write();

    allocator.deallocate(addr, page_count);

    allocator.check_slab_reservations();
}

/// Map iomem (ie: physical area not managed by phys_allocator) into kvm space.
///
/// # Safety
/// The iomem has currently no allocator, it is unchecked.
pub unsafe fn allocate_iomem(
    phys_addr: PhysAddr,
    page_count: usize,
    perms: Permissions,
) -> Result<VirtAddr, AllocatorError> {
    let mut allocator = ALLOCATOR.write();

    let res = allocator.allocate_iomem(phys_addr, page_count, perms);

    if res.is_ok() {
        allocator.check_slab_reservations();
    }

    res
}

pub fn deallocate_iomem(addr: VirtAddr, page_count: usize) {
    let mut allocator = ALLOCATOR.write();

    allocator.deallocate_iomem(addr, page_count);

    allocator.check_slab_reservations();
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
                debug!("Add page to buddy node allocator reservation");

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
                let to_reclaim = count - 1;
                debug!("Reclaim {to_reclaim} pages to buddy node allocator reservation");

                let mut dealloc = |ptr: *mut _| {
                    main_allocator.deallocate(VirtAddr::from_ptr(ptr), 1);
                };

                let reclaimed = self.allocator.try_reclaim_pages(to_reclaim, &mut dealloc);
                assert!(reclaimed == to_reclaim);
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
