use core::{mem, ops::Range, ptr};
use log::{info, trace};

use x86_64::{
    instructions::tlb,
    registers::{
        control::{Cr3, Cr3Flags},
        model_specific::{Efer, EferFlags},
    },
    structures::paging::{
        mapper::{CleanUp, FlagUpdateError, MapToError, MapperFlush, TranslateResult, UnmapError},
        page_table::PageTableEntry,
        FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, Page, PageSize, PageTable,
        PageTableFlags, PageTableIndex, PhysFrame, Size1GiB, Size2MiB, Size4KiB, Translate,
    },
    PhysAddr, VirtAddr,
};

use super::{
    access_phys,
    config::{KERNEL_START, PAGE_SIZE},
    phys::{self, AllocatorError, FrameRef},
};

pub use syscalls::Permissions;

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

static mut PHYSICAL_MAPPING_ADDRESS: VirtAddr = VirtAddr::zero();

pub static mut KERNEL_ADDRESS_SPACE: AddressSpace = AddressSpace {
    page_table: ptr::null_mut(),
};

// Keep the initial kernel stack L4 index while booting
// Filled at paging initialization
// Used while after switch to another stack to drop this initial stack
static mut INITIAL_KERNEL_STACK_L4_INDEX: Option<(PageTableIndex, Range<VirtAddr>)> = None;

// Keep ramskdisk while booting
// Filled at paging initialization
// Used while init process is loaded
static mut INITIAL_RAMDISK_L4_INDEX: Option<(PageTableIndex, Range<VirtAddr>)> = None;

pub fn create_adress_space() -> Result<AddressSpace, AllocatorError> {
    // Create new empty page table

    unsafe {
        let mut frame = phys::allocate()?;
        access_phys(&frame).fill(0);

        let virt = PHYSICAL_MAPPING_ADDRESS + frame.borrow().as_u64();

        let address_space = AddressSpace {
            page_table: virt.as_mut_ptr(),
        };

        // Clone L4 entries from kernel address space
        let page_table = address_space.get_page_table();
        for (index, entry) in KERNEL_ADDRESS_SPACE.get_page_table().iter().enumerate() {
            if !entry.is_unused() {
                page_table[index] = entry.clone();
            }
        }

        Ok(address_space)
    }
}

pub fn init(phys_mapping: VirtAddr, ramdisk: &Range<usize>) {
    unsafe {
        Efer::update(|flags| {
            *flags |= EferFlags::NO_EXECUTE_ENABLE;
        });

        PHYSICAL_MAPPING_ADDRESS = phys_mapping;
        KERNEL_ADDRESS_SPACE.page_table = get_current_page_table();

        // Only keep mapping of:
        // - Kernel
        // - Physical memory mapping
        // - Kernel stack (until we can switch context)

        // Ensure physical memory is really marked as used for this 3 entries + set proper flags
        let stack_var = 42;
        let page_table = KERNEL_ADDRESS_SPACE.get_page_table();

        let (kernel_l4_index, kernel_start, kernel_end) =
            prepare_mapping(page_table, KERNEL_START, true);
        info!(
            "Kernel: {:?} -> {:?} (size={})",
            kernel_start,
            kernel_end,
            kernel_end - kernel_start
        );

        let (phys_mapping_l4_index, phys_mapping_start, phys_mapping_end) =
            prepare_mapping(page_table, PHYSICAL_MAPPING_ADDRESS, false);
        info!(
            "Physical mapping: {:?} -> {:?} (size={})",
            phys_mapping_start,
            phys_mapping_end,
            phys_mapping_end - phys_mapping_start
        );

        let (kernel_stack_l4_index, kernel_stack_start, kernel_stack_end) =
            prepare_mapping(page_table, VirtAddr::from_ptr(&stack_var), true);
        info!(
            "Kernel stack: {:?} -> {:?} (size={})",
            kernel_stack_start,
            kernel_stack_end,
            kernel_stack_end - kernel_stack_start
        );

        let (ramdisk_l4_index, ramdisk_start, ramdisk_end) =
            prepare_mapping(page_table, VirtAddr::new(ramdisk.start as u64), true);
        info!(
            "Ramdisk: {:?} -> {:?} (size={})",
            ramdisk_start,
            ramdisk_end,
            ramdisk_end - ramdisk_start
        );

        assert!(ramdisk_start.as_u64() as usize == ramdisk.start);
        assert!(ramdisk_end.as_u64() as usize >= ramdisk.end);

        INITIAL_KERNEL_STACK_L4_INDEX =
            Some((kernel_stack_l4_index, kernel_stack_start..kernel_stack_end));

        INITIAL_RAMDISK_L4_INDEX = Some((ramdisk_l4_index, ramdisk_start..ramdisk_end));

        // Drop everything else

        // Note: bootloader prepare each memory component in its own l4 index so we can keep our 3 indexes and drop any other
        // Note: framebuffer is mapped with memory outside of physical memory
        for (l4_index, l4_entry) in page_table.iter_mut().enumerate() {
            if l4_index == kernel_l4_index.into()
                || l4_index == phys_mapping_l4_index.into()
                || l4_index == kernel_stack_l4_index.into()
                || l4_index == ramdisk_l4_index.into()
            {
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

/// Drop the initial kernel stack
///
/// Note: no process address space must exist at this time.
pub fn drop_initial_kernel_stack() {
    unsafe {
        let (l4_index, stack_range) = INITIAL_KERNEL_STACK_L4_INDEX
            .take()
            .expect("INITIAL_KERNEL_STACK_L4_INDEX not set");

        // Drop hierarchy in kernel address space
        let kernel_page_table = KERNEL_ADDRESS_SPACE.get_page_table();
        assert!(get_current_page_table() as *const _ == kernel_page_table as *const _);

        drop_mapping(&mut kernel_page_table[l4_index]);

        // Invalidate all stack pages
        for page_addr in stack_range.step_by(PAGE_SIZE) {
            tlb::flush(page_addr);
        }
    }
}

/// Drop the initial ramdisk
///
/// Note: no process address space must exist at this time.
pub fn drop_initial_ramdisk() {
    unsafe {
        let (l4_index, ramdisk_range) = INITIAL_RAMDISK_L4_INDEX
            .take()
            .expect("INITIAL_RAMDISK_L4_INDEX not set");

        // Drop hierarchy in kernel address space
        let kernel_page_table = KERNEL_ADDRESS_SPACE.get_page_table();
        assert!(get_current_page_table() as *const _ == kernel_page_table as *const _);

        drop_mapping(&mut kernel_page_table[l4_index]);

        // Invalidate all stack pages
        for page_addr in ramdisk_range.step_by(PAGE_SIZE) {
            tlb::flush(page_addr);
        }
    }
}

/// Preparation:
/// - verify that the mapped physical pages are marked as used
/// - add the GLOBAL flag and remove the USER_ACCESSIBLE flag on the entries
///
/// Take a pointer into the region, and get its level 4 index to process
///
/// Returns the begin/end of the mapped region in the level 4 index
unsafe fn prepare_mapping(
    page_table: &mut PageTable,
    pointer: VirtAddr,
    check_frame_refs: bool,
) -> (PageTableIndex, VirtAddr, VirtAddr) {
    let l4_index = Page::<Size4KiB>::containing_address(pointer).p4_index();
    let l4_entry = &mut page_table[l4_index];
    debug_assert!(!l4_entry.is_unused());

    let mut begin = VirtAddr::new_truncate(!0u64);
    let mut end = VirtAddr::new_truncate(0);

    fix_flags(l4_entry);

    debug_assert!(
        phys::used(l4_entry.addr()),
        "frame {:?} used by PageTable is not marked as used.",
        l4_entry.addr()
    );
    for (l3_index, l3_entry) in phys_frame_to_page_table(l4_entry.addr())
        .iter_mut()
        .enumerate()
    {
        if l3_entry.is_unused() {
            continue;
        }

        fix_flags(l3_entry);

        if l3_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            if !prepare_page::<Size1GiB>(
                l4_index.into(),
                l3_index,
                0,
                0,
                l3_entry.addr(),
                &mut begin,
                &mut end,
                check_frame_refs,
            ) {
                // Drop the entry if it is not valid
                l3_entry.set_unused();
                // TODO: drop the whole hierarchy if unused
            }
            continue;
        }

        debug_assert!(
            phys::used(l3_entry.addr()),
            "frame {:?} used by PageTable is not marked as used.",
            l3_entry.addr()
        );
        for (l2_index, l2_entry) in phys_frame_to_page_table(l3_entry.addr())
            .iter_mut()
            .enumerate()
        {
            if l2_entry.is_unused() {
                continue;
            }

            fix_flags(l2_entry);

            if l2_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                if !prepare_page::<Size2MiB>(
                    l4_index.into(),
                    l3_index,
                    l2_index,
                    0,
                    l2_entry.addr(),
                    &mut begin,
                    &mut end,
                    check_frame_refs,
                ) {
                    // Drop the entry if it is not valid
                    l2_entry.set_unused();
                    // TODO: drop the whole hierarchy if unused
                }
                continue;
            }

            debug_assert!(
                phys::used(l2_entry.addr()),
                "frame {:?} used by PageTable is not marked as used.",
                l2_entry.addr()
            );
            for (l1_index, l1_entry) in phys_frame_to_page_table(l2_entry.addr())
                .iter_mut()
                .enumerate()
            {
                if l1_entry.is_unused() {
                    continue;
                }

                fix_flags(l1_entry);

                if !prepare_page::<Size4KiB>(
                    l4_index.into(),
                    l3_index,
                    l2_index,
                    l1_index,
                    l1_entry.addr(),
                    &mut begin,
                    &mut end,
                    check_frame_refs,
                ) {
                    // Drop the entry if it is not valid
                    l1_entry.set_unused();
                    // TODO: drop the whole hierarchy if unused
                }
            }
        }
    }

    return (l4_index, begin, end);
}

fn prepare_page<S: PageSize>(
    l4_index: usize,
    l3_index: usize,
    l2_index: usize,
    l1_index: usize,
    frame: PhysAddr,
    begin: &mut VirtAddr,
    end: &mut VirtAddr,
    check_frame_ref: bool,
) -> bool {
    let address = Page::from_page_table_indices(
        PageTableIndex::new(u16::try_from(l4_index).unwrap()),
        PageTableIndex::new(u16::try_from(l3_index).unwrap()),
        PageTableIndex::new(u16::try_from(l2_index).unwrap()),
        PageTableIndex::new(u16::try_from(l1_index).unwrap()),
    )
    .start_address();

    if check_frame_ref {
        debug_assert!(
            phys::check_frame(frame),
            "frame {frame:?} (address={address:?}) is not valid."
        );
    } else {
        // The bootloader can create a bigger phys mem mapping than the phys mem size.
        //
        // Entry cannot point outside of physical memory.
        // If this is the case, remove the entry

        if !phys::check_frame(frame) {
            trace!("Frame {frame:?} (address={address:?}) is not valid, dropping the mapping.");
            return false;
        }
    }

    if address < *begin {
        *begin = address;
    }

    if address + PAGE_SIZE as u64 > *end {
        *end = address + PAGE_SIZE as u64;
    }

    if check_frame_ref && !phys::used(frame) {
        // It seems sometimes pages used for pagetable by bootloader are not properly marked as used
        trace!("Frame {:?} was not marked as used.", frame);
        unsafe {
            phys::allocate_at(frame).expect("Cannot use frame").borrow();
        }
    }

    true
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

        assert!(
            !l3_entry.flags().contains(PageTableFlags::HUGE_PAGE),
            "HUGE_PAGE not handled"
        );

        for l2_entry in phys_frame_to_page_table(l3_entry.addr()).iter_mut() {
            if l2_entry.is_unused() {
                continue;
            }

            assert!(
                !l2_entry.flags().contains(PageTableFlags::HUGE_PAGE),
                "HUGE_PAGE not handled"
            );

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
#[derive(Debug)]
pub struct AddressSpace {
    page_table: *mut PageTable,
}

unsafe impl Sync for AddressSpace {}
unsafe impl Send for AddressSpace {}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        unsafe {
            // All user mappings should have been dropped.
            // So the address space at this time should only contains kernel stuff.
            // So the root page table should be the same than the kernel one
            let page_table = self.get_page_table();
            for (index, entry) in KERNEL_ADDRESS_SPACE.get_page_table().iter().enumerate() {
                let kentry: u64 = mem::transmute_copy(entry);
                let uentry: u64 = mem::transmute_copy(&page_table[index]);
                assert!(
                    kentry == uentry,
                    "Page diff at {}: kernel={:?}, process={:?}",
                    index,
                    entry,
                    &page_table[index]
                );
            }

            // Drop the root page table
            let phys_addr = VirtAddr::from_ptr(self.page_table) - PHYSICAL_MAPPING_ADDRESS;
            let frame = FrameRef::unborrow(PhysAddr::new(phys_addr));

            // Explicit
            mem::drop(frame);
        }
    }
}

impl AddressSpace {
    fn get_page_table<'a>(&'a self) -> &'a mut PageTable {
        // We scoped the lifetime of PageTable ref to lifetime of self, so we are now safe
        unsafe { &mut *self.page_table }
    }

    fn create_manager<'a>(&'a self) -> OffsetPageTable<'a> {
        unsafe { OffsetPageTable::new(self.get_page_table(), PHYSICAL_MAPPING_ADDRESS) }
    }

    pub unsafe fn map(
        &mut self,
        addr: VirtAddr,
        phys_addr: PhysAddr,
        permissions: Permissions,
        additional_flags: Option<AdditionalFlags>,
    ) -> Result<(), MapToError<Size4KiB>> {
        assert!(addr.is_aligned(PAGE_SIZE as u64));
        assert!(phys_addr.is_aligned(PAGE_SIZE as u64));

        let mut manager = self.create_manager();
        let mut frame_allocator = FrameAllocatorImpl::default();

        let flusher = manager.map_to_with_table_flags(
            Page::<Size4KiB>::from_start_address_unchecked(addr),
            PhysFrame::from_start_address_unchecked(phys_addr),
            create_flags(addr, permissions, additional_flags),
            create_parent_flags(addr),
            &mut frame_allocator,
        )?;

        self.flush(addr, flusher);

        Ok(())
    }

    pub unsafe fn remap(
        &mut self,
        addr: VirtAddr,
        phys_addr: PhysAddr,
        permissions: Permissions,
        additional_flags: Option<AdditionalFlags>,
    ) -> Result<FrameRef, UnmapError> {
        assert!(addr.is_aligned(PAGE_SIZE as u64));
        assert!(phys_addr.is_aligned(PAGE_SIZE as u64));

        let mut manager = self.create_manager();
        let mut frame_allocator = FrameAllocatorImpl::default();

        let (unmapped_frame, flusher) =
            manager.unmap(Page::<Size4KiB>::from_start_address_unchecked(addr))?;

        // Ignore this flush, we will apply the 'map' flush
        flusher.ignore();

        let flusher = manager
            .map_to_with_table_flags(
                Page::<Size4KiB>::from_start_address_unchecked(addr),
                PhysFrame::from_start_address_unchecked(phys_addr),
                create_flags(addr, permissions, additional_flags),
                create_parent_flags(addr),
                &mut frame_allocator,
            )
            .unwrap();

        // Note: all errors from map should not happen:
        // - FrameAllocationFailed: we just unmapped the page, so no page table creation needed
        // - ParentEntryHugePage: we just unmapped the page successfully, so not possible
        // - PageAlreadyMapped: we just unmapped the page, so cannot be mapped anymore

        self.flush(addr, flusher);

        Ok(FrameRef::unborrow(unmapped_frame.start_address()))
    }

    pub unsafe fn update_permissions(
        &mut self,
        addr: VirtAddr,
        permissions: Permissions,
    ) -> Result<(), FlagUpdateError> {
        assert!(addr.is_aligned(PAGE_SIZE as u64));

        let mut manager = self.create_manager();
        // Retrieve additional flags to keep them
        let (_, _, additional_flags) = self.get_infos(addr);

        let flusher = manager.update_flags(
            Page::<Size4KiB>::from_start_address_unchecked(addr),
            create_flags(addr, permissions, Some(additional_flags)),
        )?;

        self.flush(addr, flusher);

        Ok(())
    }

    pub unsafe fn unmap(&mut self, addr: VirtAddr) -> Result<PhysAddr, UnmapError> {
        assert!(addr.is_aligned(PAGE_SIZE as u64));

        let mut manager = self.create_manager();
        let mut frame_allocator = FrameAllocatorImpl::default();
        let page = Page::<Size4KiB>::from_start_address_unchecked(addr);

        let (unmapped_frame, flusher) = manager.unmap(page)?;

        self.flush(addr, flusher);

        manager.clean_up_addr_range(Page::range_inclusive(page, page), &mut frame_allocator);

        Ok(unmapped_frame.start_address())
    }

    pub unsafe fn get_infos(
        &self,
        addr: VirtAddr,
    ) -> (Option<PhysAddr>, Permissions, AdditionalFlags) {
        assert!(addr.is_aligned(PAGE_SIZE as u64));

        let manager = self.create_manager();

        match manager.translate(addr) {
            TranslateResult::Mapped {
                frame,
                offset: _,
                flags,
            } => {
                let mut perm = Permissions::READ;
                let mut additional_flags = AdditionalFlags::new();

                if flags.contains(PageTableFlags::WRITABLE) {
                    perm |= Permissions::WRITE;
                }

                if !flags.contains(PageTableFlags::NO_EXECUTE) {
                    perm |= Permissions::EXECUTE;
                }

                additional_flags.write_through(flags.contains(PageTableFlags::WRITE_THROUGH));
                additional_flags.no_cache(flags.contains(PageTableFlags::NO_CACHE));

                (Some(frame.start_address()), perm, additional_flags)
            }

            TranslateResult::NotMapped => (None, Permissions::NONE, AdditionalFlags::new()),

            TranslateResult::InvalidFrameAddress(frame) => {
                panic!("Invalid phys page {frame:?} mapped at {addr:?}");
            }
        }
    }

    fn flush(&self, addr: VirtAddr, flusher: MapperFlush<Size4KiB>) {
        // Always flush kernel space change.
        // Only change user space change if the address space is currently loaded.
        if !is_user_address(addr) || self.is_active() {
            flusher.flush();
        } else {
            flusher.ignore();
        }
    }

    fn is_active(&self) -> bool {
        self.page_table == unsafe { get_current_page_table() }
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

#[inline]
fn is_user_address(addr: VirtAddr) -> bool {
    addr < KERNEL_START
}

#[derive(Debug, Clone, Copy)]
pub struct AdditionalFlags {
    pub write_through: Option<bool>,
    pub no_cache: Option<bool>,
}

impl AdditionalFlags {
    pub const fn new() -> Self {
        Self {
            write_through: None,
            no_cache: None,
        }
    }

    pub fn write_through(&mut self, value: bool) -> &mut Self {
        self.write_through = Some(value);
        self
    }

    pub fn no_cache(&mut self, value: bool) -> &mut Self {
        self.no_cache = Some(value);
        self
    }
}

#[inline]
fn create_flags(
    addr: VirtAddr,
    permissions: Permissions,
    additional_flags: Option<AdditionalFlags>,
) -> PageTableFlags {
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

    if let Some(additional_flags) = additional_flags {
        if let Some(true) = additional_flags.write_through {
            flags |= PageTableFlags::WRITE_THROUGH;
        }

        if let Some(true) = additional_flags.no_cache {
            flags |= PageTableFlags::NO_CACHE;
        }
    }

    return flags;
}

#[inline]
fn create_parent_flags(addr: VirtAddr) -> PageTableFlags {
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    if is_user_address(addr) {
        flags |= PageTableFlags::USER_ACCESSIBLE;
    } else {
        flags |= PageTableFlags::GLOBAL;
    }

    return flags;
}

#[inline]
unsafe fn phys_frame_to_page_table(frame: PhysAddr) -> &'static mut PageTable {
    let virt = PHYSICAL_MAPPING_ADDRESS + frame.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

#[inline]
unsafe fn page_table_to_phys_frame(page_table: &PageTable) -> PhysAddr {
    let page_table_ptr: *const PageTable = page_table;
    return PhysAddr::new(VirtAddr::from_ptr(page_table_ptr) - PHYSICAL_MAPPING_ADDRESS);
}

/// Helper to get a virtual address from physical one.
///
/// Return a virtual address that corresponds to a view of the physical address, using the physical memory mapping
#[inline]
pub fn phys_to_virt(addr: PhysAddr) -> VirtAddr {
    return unsafe { PHYSICAL_MAPPING_ADDRESS } + addr.as_u64();
}
