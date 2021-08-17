use bootloader::BootInfo;
use x86_64::structures::paging::{PageSize, Size4KiB};

pub const PAGE_SIZE: usize = Size4KiB::SIZE as usize;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 5;
pub const VM_SIZE: u64 = 1u64 << 48;
pub const VM_SPLIT: u64 = VM_SIZE / 2;

mod frame_allocator;
mod paging;
mod phys_view;

pub fn init(boot_info: &'static BootInfo) {
    phys_view::init(boot_info);
    frame_allocator::init(boot_info);
    paging::init();
}
