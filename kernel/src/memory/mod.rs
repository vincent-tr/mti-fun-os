use bootloader::BootInfo;
use x86_64::{
    structures::paging::{mapper::OffsetPageTable, PageSize, PageTable, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::println;

pub const PAGE_SIZE: usize = Size4KiB::SIZE as usize;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 5;

/// `Offseted` mapping of physical memory starting at this offset in virtual memory
static mut physical_memory_offset: u64 = 0;

// WARNING: this is static, so only valid until physical_memory_offset changes, or cr3 changes
static mut MAPPER: Option<OffsetPageTable> = Option::None;

pub fn init(boot_info: &'static BootInfo) {
    println!(
        "Physical memory offset: {:#X}",
        boot_info.physical_memory_offset
    );

    unsafe {
        physical_memory_offset = boot_info.physical_memory_offset;
    }

    for region in boot_info.memory_map.iter() {
        println!("Region: {:?} {:?}", region.region_type, region.range);
    }

    unsafe {
        let l4_page_table = active_level_4_table();
        MAPPER = Some(OffsetPageTable::new(l4_page_table, VirtAddr::new(physical_memory_offset)));
    }
}

pub fn to_phys(virt: VirtAddr) -> PhysAddr {
    return PhysAddr::new(virt.as_u64() - unsafe { physical_memory_offset });
}

pub fn to_virt_view(phys: PhysAddr) -> VirtAddr {
    return VirtAddr::new(phys.as_u64() + unsafe { physical_memory_offset });
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
unsafe fn active_level_4_table() -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let addr = to_virt_view(level_4_table_frame.start_address());
    let page_table_ptr: *mut PageTable = addr.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}
