use bootloader::BootInfo;
use x86_64::{PhysAddr, VirtAddr};

use crate::println;

/// `Offseted` mapping of physical memory starting at this offset in virtual memory
static mut OFFSET: u64 = 0;
static mut SIZE: u64 = 0;

pub fn init(boot_info: &'static BootInfo) {
    unsafe {
        OFFSET = boot_info.physical_memory_offset;
    }

    for region in boot_info.memory_map.iter() {
        let region_end = region.range.end_addr();
        if unsafe { SIZE } < region_end {
            unsafe { SIZE = region_end };
        }
    }

    println!(
        "Physical memory map: {:#X} -> {:#X} (size={})",
        unsafe { OFFSET },
        unsafe { OFFSET + SIZE },
        unsafe { SIZE }
    );
}

pub fn to_phys(virt: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt.as_u64() - unsafe { OFFSET })
}

pub fn to_virt_view(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + unsafe { OFFSET })
}

pub fn physical_memory_size() -> u64 {
    unsafe { SIZE }
}
