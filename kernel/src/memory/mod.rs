use bootloader::BootInfo;
use x86_64::{
    structures::paging::{mapper::OffsetPageTable, PageSize, PageTable, Size4KiB, Translate},
    PhysAddr, VirtAddr,
};

use crate::println;

pub const PAGE_SIZE: usize = Size4KiB::SIZE as usize;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 5;

mod frame_allocator;
mod paging;
mod phys_view;

pub fn init(boot_info: &'static BootInfo) {
    phys_view::init(boot_info);
    frame_allocator::init(boot_info, &mut [] as &mut [u32]);

    for region in boot_info.memory_map.iter() {
        println!("Region: {:?} {:?}", region.region_type, region.range);
    }

    unsafe {
        let l4_page_table = active_level_4_table();
        let mut mapper = OffsetPageTable::new(l4_page_table, phys_view::to_virt_view(PhysAddr::new(0)));

        let l2_entry_range: u64 = 4096 * 512;
        let l3_entry_range: u64 = l2_entry_range * 512;
        let l4_entry_range: u64 = l3_entry_range * 512;

        for (l4_index, l4_entry) in active_level_4_table().iter().enumerate() {
            if l4_entry.is_unused() {
                continue;
            }

            let l4_begin = l4_index as u64 * l4_entry_range;
            let l4_end = (l4_index as u64 + 1) * l4_entry_range;

            println!("Virtual L4 {:#X} -> {:#X}", l4_begin, l4_end);

            let l3_page_table_ptr: *mut PageTable = phys_view::to_virt_view(l4_entry.addr()).as_mut_ptr();
            let l3_page_table = &*l3_page_table_ptr;

            for (l3_index, l3_entry) in l3_page_table.iter().enumerate() {
                if l3_entry.is_unused() {
                    continue;
                }

                let l3_begin = l3_index as u64 * l3_entry_range;
                let l3_end = (l3_index as u64 + 1) * l3_entry_range;

                println!(
                    "Virtual   L3 {:#X} -> {:#X}",
                    l3_begin + l4_begin,
                    l3_end + l4_begin
                );

                let maybe_phys =
                    mapper.translate_addr(VirtAddr::new(if l3_begin + l4_begin == 0 {
                        0x1000u64
                    } else {
                        l3_begin + l4_begin
                    }));
                if let Some(phys) = maybe_phys {
                    println!("Phys {:?}", phys);
                }
            }

            let stack_value = 42;
            let stack_ptr: *const i32 = &stack_value;

            println!("Stack ptr {:p}", stack_ptr);
        }
    }
}

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table() -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let addr = phys_view::to_virt_view(level_4_table_frame.start_address());
    let page_table_ptr: *mut PageTable = addr.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}
