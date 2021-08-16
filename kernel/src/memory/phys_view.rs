use bootloader::BootInfo;
use x86_64::{PhysAddr, VirtAddr};

use crate::println;

/// `Offseted` mapping of physical memory starting at this offset in virtual memory
static mut physical_memory_offset: u64 = 0;
static mut physical_memory_max: u64 = 0;

pub fn init(boot_info: &'static BootInfo) {
    println!(
        "Physical memory offset: {:#X}",
        boot_info.physical_memory_offset
    );

    unsafe {
        physical_memory_offset = boot_info.physical_memory_offset;
    }

    for region in boot_info.memory_map.iter() {
        let region_end = region.range.end_addr();
        if unsafe { physical_memory_max } < region_end {
            unsafe { physical_memory_max = region_end };
        }
    }
}

pub fn to_phys(virt: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt.as_u64() - unsafe { physical_memory_offset })
}

pub fn to_virt_view(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + unsafe { physical_memory_offset })
}

pub fn physical_memory_size() -> u64 {
    unsafe { physical_memory_max }
}
