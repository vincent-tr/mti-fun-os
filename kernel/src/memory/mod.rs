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
    frame_allocator::init(boot_info);
    paging::init();
}
