use x86_64::VirtAddr;

pub const PAGE_SIZE: u64 = 4096;

pub const KERNEL_START: VirtAddr = VirtAddr::new_truncate(0xFFFF_8000_0000_0000);

pub const VMALLOC_START: VirtAddr = VirtAddr::new_truncate(0xFFFF_8000_4000_0000);
pub const VMALLOC_END: VirtAddr = VirtAddr::new_truncate(0xFFFF_8080_0000_0000);
