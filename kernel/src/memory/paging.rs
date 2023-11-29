use bitflags::bitflags;
use log::{info, warn};
use core::{ptr::null_mut, mem};

use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        mapper::{CleanUp, FlagUpdateError, MapToError, UnmapError},
        FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, Page, PageTable,
        PageTableFlags, PhysFrame, Size4KiB, page_table::PageTableEntry, PageTableIndex, PageSize, Size1GiB, Size2MiB,
    },
    PhysAddr, VirtAddr, instructions::tlb,
};

use super::phys::{self, FrameRef, PAGE_SIZE};

/*

64 bits virtual address

64         48         39         30         21         12         0
   Unused   | Level 4  | Level 3  | Level 2  | Level 1  | Page Offset

bits 48 -> 64 must match bit 47

Level 4 entry size: 0x80_0000_0000 512G
Level 3 entry size: 0x4000_0000      1G
Level 2 entry size: 0x20_0000        2M
Level 1 entry size: 0x1000           4K

---

Bootloader state:
INFO - Kernel      0xFFFF_8000_0000_0000
INFO - Phys mem    0xFFFF_8080_0000_0000
INFO - Stack       0xFFFF_8100_0000_0000
INFO - Framebuffer 0xFFFF_8180_0000_0000
INFO - Boot info   0xFFFF_8200_0000_0000

After full initialization (drop of initial kernel stack):
- Page Table L4 entry #256 (0xFFFF_8000_0000_0000):  Kernel + vmalloc
- Page Table L4 entry #257 (0xFFFF_8080_0000_0000):  Physical memory mapping
This 2 entries needs to be copied to all Page Tables created for user processes

*/

bitflags! {
    /// Possible paging permissions
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct Permissions: u64 {
        /// Page can be read
        const READ = 1 << 0;

        /// Page can be written
        const WRITE = 1 << 1;

        /// Page can be executed
        const EXECUTE = 1 << 2;
    }
}

pub const KERNEL_START: VirtAddr = VirtAddr::new_truncate(0xFFFF_8000_0000_0000);

static mut PHYSICAL_MAPPING_ADDRESS: VirtAddr = VirtAddr::zero();

static mut KERNEL_ADDRESS_SPACE: AddressSpace = AddressSpace { page_table: null_mut() };

pub fn init(phys_mapping: VirtAddr) {
    unsafe {
        PHYSICAL_MAPPING_ADDRESS = phys_mapping;
        KERNEL_ADDRESS_SPACE.page_table = get_current_page_table();

        // Only keep mapping of:
        // - Kernel
        // - Physical memory mapping
        // - Kernel stack (until we can switch context)
        
        // Ensure physical memory is really marked as used for this 3 entries + set proper flags
        let stack_var = 42;
        let page_table = KERNEL_ADDRESS_SPACE.get_page_table_mut();
        
        let (kernel_l4_index, kernel_start, kernel_end) = prepare_mapping(page_table, KERNEL_START, true);
        info!("Kernel: {:?} -> {:?} (size={})", kernel_start, kernel_end, kernel_end - kernel_start);

        let (phys_mapping_l4_index, phys_mapping_start, phys_mapping_end) = prepare_mapping(page_table, PHYSICAL_MAPPING_ADDRESS, false);
        info!("Physical mapping: {:?} -> {:?} (size={})", phys_mapping_start, phys_mapping_end, phys_mapping_end - phys_mapping_start);

        let (kernel_stack_l4_index, kernel_stack_start, kernel_stack_end) = prepare_mapping(page_table, VirtAddr::from_ptr(&stack_var), true);
        info!("Kernel stack: {:?} -> {:?} (size={})", kernel_stack_start, kernel_stack_end, kernel_stack_end - kernel_stack_start);

        // Drop everything else

        // Note: bootloader prepare each memory component in its own l4 index so we can keep our 3 indexes and drop any other
        // Note: framebuffer is mapped with memory outside of physical memory
        for (l4_index, l4_entry) in page_table.iter_mut().enumerate() {
            if l4_index == kernel_l4_index.into() || l4_index == phys_mapping_l4_index.into() || l4_index == kernel_stack_l4_index.into() {
                continue;
            }

            if l4_entry.is_unused() {
                continue;
            }

            drop_mapping(l4_entry);
        }

        tlb::flush_all();
    }
}

/// Preparation:
/// - verify that the mapped physical pages are marked as used
/// - add the GLOBAL flag and remove the USER_ACCESSIBLE flag on the entries
/// 
/// Take a pointer into the region, and get its level 4 index to process
/// 
/// Returns the begin/end of the mapped region in the level 4 index
unsafe fn prepare_mapping(page_table: &mut PageTable, pointer: VirtAddr, check_frame_refs: bool) -> (PageTableIndex, VirtAddr, VirtAddr) {
    let l4_index = Page::<Size4KiB>::containing_address(pointer).p4_index();
    let l4_entry = &mut page_table[l4_index];
    debug_assert!(!l4_entry.is_unused());

    let mut begin = VirtAddr::new_truncate(!0u64);
    let mut end = VirtAddr::new_truncate(0);

    fix_flags(l4_entry);

    debug_assert!(phys::used(l4_entry.addr()), "frame {:?} used by PageTable is not marked as used.", l4_entry.addr());
    for (l3_index, l3_entry) in phys_frame_to_page_table(l4_entry.addr()).iter_mut().enumerate() {
        if l3_entry.is_unused() {
            continue;
        }

        fix_flags(l3_entry);

        if l3_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            prepare_page::<Size1GiB>(l4_index.into(), l3_index, 0, 0, l3_entry.addr(), &mut begin, &mut end, check_frame_refs);
            continue;
        }

        debug_assert!(phys::used(l3_entry.addr()), "frame {:?} used by PageTable is not marked as used.", l3_entry.addr());
        for (l2_index, l2_entry) in phys_frame_to_page_table(l3_entry.addr()).iter_mut().enumerate() {
            if l2_entry.is_unused() {
                continue;
            }

            fix_flags(l2_entry);

            if l2_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                prepare_page::<Size2MiB>(l4_index.into(), l3_index, l2_index, 0, l2_entry.addr(), &mut begin, &mut end, check_frame_refs);
                continue;
            }

            debug_assert!(phys::used(l2_entry.addr()), "frame {:?} used by PageTable is not marked as used.", l2_entry.addr());
            for (l1_index, l1_entry) in phys_frame_to_page_table(l2_entry.addr()).iter_mut().enumerate() {
                if l1_entry.is_unused() {
                    continue;
                }

                fix_flags(l1_entry);

                prepare_page::<Size4KiB>(l4_index.into(), l3_index, l2_index, l1_index, l1_entry.addr(), &mut begin, &mut end, check_frame_refs);
            }
        }
    }

    return (l4_index, begin, end)
}

fn prepare_page<S: PageSize>(l4_index: usize, l3_index: usize, l2_index: usize, l1_index: usize, frame: PhysAddr, begin: &mut VirtAddr, end: &mut VirtAddr, check_frame_ref: bool) {
    let address = Page::from_page_table_indices(
        PageTableIndex::new(u16::try_from(l4_index).unwrap()), 
        PageTableIndex::new(u16::try_from(l3_index).unwrap()), 
        PageTableIndex::new(u16::try_from(l2_index).unwrap()), 
        PageTableIndex::new(u16::try_from(l1_index).unwrap())
    ).start_address();

    debug_assert!(phys::check_frame(frame), "frame {frame:?} (address={address:?}) is not valid.");

    if address < *begin {
        *begin = address;
    }

    if address + PAGE_SIZE > *end {
        *end = address + PAGE_SIZE;
    }

    if check_frame_ref && !phys::used(frame) {
        warn!("Frame {:?} was not marked as used.", frame);
        unsafe {
            phys::allocate_at(frame).expect("Cannot use frame").borrow();
        }
    }
}

fn fix_flags(entry: &mut PageTableEntry) {
    let mut flags = entry.flags();

    flags |= PageTableFlags::GLOBAL;
    flags &= !PageTableFlags::USER_ACCESSIBLE;

    entry.set_flags(flags);
}

unsafe fn drop_mapping(l4_entry: &mut PageTableEntry) {

    for l3_entry in phys_frame_to_page_table(l4_entry.addr()).iter_mut() {
        if l3_entry.is_unused() {
            continue;
        }

        assert!(!l3_entry.flags().contains(PageTableFlags::HUGE_PAGE), "HUGE_PAGE not handled");

        for l2_entry in phys_frame_to_page_table(l3_entry.addr()).iter_mut() {
            if l2_entry.is_unused() {
                continue;
            }

            assert!(!l2_entry.flags().contains(PageTableFlags::HUGE_PAGE), "HUGE_PAGE not handled");

            for l1_entry in phys_frame_to_page_table(l2_entry.addr()).iter_mut() {
                if l1_entry.is_unused() {
                    continue;
                }
                
                let frame = l1_entry.addr();
                // Note: framebuffer maps memory outside physical memory
                // ignore it.
                if phys::check_frame(frame) && phys::used(frame) {
                    mem::drop(FrameRef::unborrow(l1_entry.addr()));
                }

                l1_entry.set_unused();
            }

            mem::drop(FrameRef::unborrow(l2_entry.addr()));
            l2_entry.set_unused();
        }

        mem::drop(FrameRef::unborrow(l3_entry.addr()));
        l3_entry.set_unused();
    }

    mem::drop(FrameRef::unborrow(l4_entry.addr()));
    l4_entry.set_unused();
}

unsafe fn get_current_page_table() -> &'static mut PageTable {
    let (frame, _) = Cr3::read();
    phys_frame_to_page_table(frame.start_address())
}

/// Install the provided address space as the current one
/// 
/// # Safety
/// 
/// If the address space is not properly setup, we are dead.
/// 
pub unsafe fn set_current_address_space(address_space: &AddressSpace) {
    let frame = page_table_to_phys_frame(address_space.get_page_table());
    Cr3::write(
        PhysFrame::from_start_address_unchecked(frame),
        Cr3Flags::empty(),
    );
}

/// Describe an address space, which is a complete 64 bits space of virtual memory.
/// 
/// Pysical pages can be mapped into the address space, and it can be setup as the current one.
pub struct AddressSpace {
    page_table: *mut PageTable,
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        todo!("TODO: drop all user entries + drop page table itself");
    }
}

impl AddressSpace {
    fn get_page_table<'a>(&'a self) -> &'a PageTable {
        // We scoped the lifetime of PageTable ref to lifetime of self, so we are now safe
        unsafe {
            & *self.page_table
        }
    }

    fn get_page_table_mut<'a>(&'a mut self) -> &'a mut PageTable {
        // We scoped the lifetime of PageTable ref to lifetime of self, so we are now safe
        unsafe {
            &mut *self.page_table
        }
    }

    fn create_manager<'a>(&'a mut self) -> OffsetPageTable<'a> {
        unsafe {
            OffsetPageTable::new(self.get_page_table_mut(), PHYSICAL_MAPPING_ADDRESS)
        }
    }

    pub unsafe fn map(
        &mut self,
        addr: VirtAddr,
        frame: &mut FrameRef,
        permissions: Permissions,
    ) -> Result<(), MapToError<Size4KiB>> {
        let mut manager = self.create_manager();
        let mut frame_allocator = FrameAllocatorImpl::default();

        let flush = manager.map_to_with_table_flags(
            Page::<Size4KiB>::from_start_address_unchecked(addr),
            PhysFrame::from_start_address_unchecked(frame.frame()),
            create_flags(addr, permissions),
            create_parent_flags(addr),
            &mut frame_allocator,
        )?;

        flush.flush();

        // only borrow on success
        frame.borrow();

        Ok(())
    }

    pub unsafe fn remap(
        &mut self,
        addr: VirtAddr,
        frame: &mut FrameRef,
        permissions: Permissions,
    ) -> Result<FrameRef, UnmapError> {
        let mut manager = self.create_manager();
        let mut frame_allocator = FrameAllocatorImpl::default();

        let (unmapped_frame, flush) =
            manager.unmap(Page::<Size4KiB>::from_start_address_unchecked(addr))?;

        // Ignore this flush, we will apply the 'map' flush
        flush.ignore();

        let flush = manager
            .map_to_with_table_flags(
                Page::<Size4KiB>::from_start_address_unchecked(addr),
                PhysFrame::from_start_address_unchecked(frame.frame()),
                create_flags(addr, permissions),
                create_parent_flags(addr),
                &mut frame_allocator,
            )
            .unwrap();

        // Note: all errors from map should not happen:
        // - FrameAllocationFailed: we just unmapped the page, so no page table creation needed
        // - ParentEntryHugePage: we just unmapped the page successfully, so not possible
        // - PageAlreadyMapped: we just unmapped the page, so cannot be mapped anymore

        flush.flush();

        // only borrow on success
        frame.borrow();

        Ok(FrameRef::unborrow(unmapped_frame.start_address()))
    }

    pub unsafe fn update_permissions(
        &mut self,
        addr: VirtAddr,
        permissions: Permissions,
    ) -> Result<(), FlagUpdateError> {
        let mut manager = self.create_manager();

        let flush = manager.update_flags(
            Page::<Size4KiB>::from_start_address_unchecked(addr),
            create_flags(addr, permissions),
        )?;

        flush.flush();

        Ok(())
    }

    pub unsafe fn unmap(&mut self, addr: VirtAddr) -> Result<FrameRef, UnmapError> {
        let mut manager = self.create_manager();
        let mut frame_allocator = FrameAllocatorImpl::default();
        let page = Page::<Size4KiB>::from_start_address_unchecked(addr);

        let (unmapped_frame, flush) = manager.unmap(page)?;

        flush.flush();

        manager.clean_up_addr_range(Page::range_inclusive(page, page), &mut frame_allocator);

        Ok(FrameRef::unborrow(unmapped_frame.start_address()))
    }
}

#[derive(Default)]
struct FrameAllocatorImpl {}

unsafe impl FrameAllocator<Size4KiB> for FrameAllocatorImpl {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if let Ok(mut frame_ref) = phys::allocate() {
            unsafe { Some(PhysFrame::from_start_address_unchecked(frame_ref.borrow())) }
        } else {
            None
        }
    }
}

impl FrameDeallocator<Size4KiB> for FrameAllocatorImpl {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame) {
        // Will be reclaimed on drop
        FrameRef::unborrow(frame.start_address());
    }
}

fn is_user_address(addr: VirtAddr) -> bool {
    addr < KERNEL_START
}

fn create_flags(addr: VirtAddr, permissions: Permissions) -> PageTableFlags {
    let mut flags = PageTableFlags::PRESENT;

    if is_user_address(addr) {
        flags |= PageTableFlags::USER_ACCESSIBLE;
    } else {
        flags |= PageTableFlags::GLOBAL;
    }

    if permissions.contains(Permissions::WRITE) {
        flags |= PageTableFlags::WRITABLE;
    }

    if !permissions.contains(Permissions::EXECUTE) {
        flags |= PageTableFlags::NO_EXECUTE;
    }

    return flags;
}

fn create_parent_flags(addr: VirtAddr) -> PageTableFlags {
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    if is_user_address(addr) {
        flags |= PageTableFlags::USER_ACCESSIBLE;
    } else {
        flags |= PageTableFlags::GLOBAL;
    }

    return flags;
}

unsafe fn phys_frame_to_page_table(frame: PhysAddr) -> &'static mut PageTable {
    let virt = PHYSICAL_MAPPING_ADDRESS + frame.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

unsafe fn page_table_to_phys_frame(page_table: &PageTable) -> PhysAddr {
    let page_table_ptr: *const PageTable = page_table;
    return PhysAddr::new(VirtAddr::from_ptr(page_table_ptr) - PHYSICAL_MAPPING_ADDRESS);
}
