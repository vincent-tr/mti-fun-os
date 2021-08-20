use spin::Mutex;
use x86_64::VirtAddr;

use super::{frame_allocator, paging, VM_SIZE, VM_SPLIT};
use crate::{error::Error, memory::PAGE_SIZE, println};

struct Region {
    start: VirtAddr;
    end: VirtAddr;

    prev: Option<&mut Region>;
    next: Option<&mut Region>;
}

struct RegionList {
    list: Option<&mut Region>;
    free_list: Option<&mut Region>;
}

struct RegionPage {
    used: u64;

    regions: [Region; 127]; // (PAGE_SIZE - sizeof(RegionPage)) / sizeof(Region)
}

struct Zst;

static LOCK: Mutex<Zst> = Mutex::new(Zst);

pub fn allocate(page_count: usize) -> Result<VirtAddr, Error> {
    unimplemented!();
}

pub fn deallocate(address: VirtAddr) {
    unimplemented!();
}