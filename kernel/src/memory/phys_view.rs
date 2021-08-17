use bootloader::BootInfo;
use x86_64::{PhysAddr, VirtAddr};

use crate::println;

/// `Offseted` mapping of physical memory starting at this offset in virtual memory
static mut offset: u64 = 0;
static mut size: u64 = 0;

pub fn init(boot_info: &'static BootInfo) {
    unsafe {
        offset = boot_info.physical_memory_offset;
    }

    for region in boot_info.memory_map.iter() {
        let region_end = region.range.end_addr();
        if unsafe { size } < region_end {
            unsafe { size = region_end };
        }
    }

    println!(
        "Physical memory map: {:#X} -> {:#X} (size={})",
        unsafe { offset },
        unsafe { offset + size },
        unsafe { size }
    );
}

pub fn to_phys(virt: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt.as_u64() - unsafe { offset })
}

pub fn to_virt_view(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + unsafe { offset })
}

pub fn physical_memory_size() -> u64 {
    unsafe { size }
}
