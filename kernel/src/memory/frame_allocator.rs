use spin::Mutex;
use x86_64::{structures::paging::PhysFrame, PhysAddr};
use bootloader::bootinfo::{BootInfo,MemoryRegionType, MemoryRegion};

use crate::error::Error;
use crate::memory::PAGE_SIZE;

// https://wiki.osdev.org/Page_Frame_Allocation

static ALLOCATOR: Mutex<Option<FrameAllocator>> = Mutex::new(Option::None);

pub fn init(boot_info: &'static BootInfo, stack: &'static mut [u32]) {
    let mut allocator = FrameAllocator::new();

    allocator.init(boot_info, stack);

    let mut locked = ALLOCATOR.lock();
    *locked = Some(allocator);
}

pub fn allocate() ->Result<PhysFrame, Error> {
    if let Some(allocator) = &mut *ALLOCATOR.lock() {
        allocator.allocate()
    } else {
        panic!("You must initialize frame allocator before using it");
    }
}

pub fn deallocate(frame: PhysFrame) {
    if let Some(allocator) = &mut *ALLOCATOR.lock() {
        allocator.deallocate(frame);
    } else {
        panic!("You must initialize frame allocator before using it");
    }
}

pub struct FrameAllocator<'a> {
    stack: &'a mut [u32],
    top: usize,
}

impl<'a> FrameAllocator<'a> {
    pub fn new() -> FrameAllocator<'a> {
        return FrameAllocator {
            stack: &mut [] as &mut [u32],
            top: 0,
        };
    }

    pub fn init(&mut self, boot_info: &'static BootInfo, stack: &'static mut [u32]) {
        self.stack = stack;

        for region in boot_info.memory_map.iter() {
            if region.region_type == MemoryRegionType::Usable {
                self.add_region(region);
            }
        }
    }

    fn add_region(&mut self, region: &MemoryRegion) {
        for frame_number in region.range.start_frame_number .. region.range.end_frame_number {
            assert!(self.top < self.stack.len());
            self.stack[self.top] = frame_number as u32;
            self.top += 1;
        }
    }

    pub fn allocate(&mut self) -> Result<PhysFrame, Error> {
        if self.top == 0 {
            return Err(Error::OutOfMemory);
        }

        self.top -= 1;
        let ref mut current = self.stack[self.top];

        let page = Self::frame_number_to_frame(*current);
        *current = 0;

        return Ok(page);
    }

    pub fn deallocate(&mut self, frame: PhysFrame) {
        self.stack[self.top] = Self::frame_to_frame_number(frame);
        self.top += 1;
    }

    fn frame_to_frame_number(frame: PhysFrame) -> u32 {
        return (frame.start_address().as_u64() as usize / PAGE_SIZE) as u32;
    }

    fn frame_number_to_frame(frame_number: u32) -> PhysFrame {
        return PhysFrame::from_start_address(PhysAddr::new(frame_number as u64 * PAGE_SIZE as u64)).unwrap();
    }
}
