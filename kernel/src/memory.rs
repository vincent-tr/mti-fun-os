use bootloader::BootInfo;
use x86_64::{
    structures::paging::{mapper::OffsetPageTable, PageSize, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::println;

pub const PAGE_SIZE: usize = Size4KiB::SIZE as usize;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 5;

// WARNING: this is static, so only valid until physical_memory_offset changes, or cr3 changes
static mut MAPPER: Option<OffsetPageTable> = Option::None;

pub fn init(boot_info: &'static BootInfo) {
    for region in boot_info.memory_map.iter() {
        println!("Region: {:?} {:?}", region.region_type, region.range);
    }

    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);

    unsafe {
        let l4_page_table = active_level_4_table(physical_memory_offset);
        MAPPER = Some(OffsetPageTable::new(l4_page_table, physical_memory_offset));
    }
}

fn mapper() -> &'static OffsetPageTable<'static> {
    unsafe { MAPPER.as_ref().unwrap() }
}

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}

// https://wiki.osdev.org/Page_Frame_Allocation
pub struct FrameAllocator<'a> {
    bitmap: &'a mut [u64],
}

impl<'a> FrameAllocator<'a> {
    pub fn allocate(&mut self) -> PhysFrame {
        for (word_index, word) in self.bitmap.iter_mut().enumerate() {
            let bit_index = word.leading_ones() as usize;
            if bit_index < 64 {
                *word |= 1u64 << bit_index;

                let page_offset = word_index * 64 + bit_index;
                let address = PhysAddr::new((page_offset * PAGE_SIZE) as u64);
                return PhysFrame::from_start_address(address).unwrap();
            }
        }

        panic!("Frame allocator: allocation failure, no more free frame");
    }

    pub fn deallocate(&mut self, frame: PhysFrame) {
        let page_offset = frame.start_address().as_u64() as usize / PAGE_SIZE;
        let word_index = page_offset / 64;
        let bit_index = (page_offset - (word_index * 64)) as usize;
        let word = unsafe { self.bitmap.get_unchecked_mut(word_index) };
        *word &= !(1u64 << bit_index);
    }
}
