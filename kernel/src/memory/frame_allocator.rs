use bootloader::bootinfo::{BootInfo, MemoryRegionType};
use core::{mem::size_of, slice};
use spin::Mutex;
use x86_64::{align_up, structures::paging::PhysFrame, PhysAddr, VirtAddr};

use super::phys_view;
use crate::{error::Error, memory::PAGE_SIZE, println};

// https://wiki.osdev.org/Page_Frame_Allocation

static ALLOCATOR: Mutex<Option<FrameAllocator>> = Mutex::new(Option::None);

pub fn init(boot_info: &'static BootInfo) {
    let mut allocator = FrameAllocator::new();

    allocator.init(boot_info);

    let mut locked = ALLOCATOR.lock();
    *locked = Some(allocator);
}

pub fn allocate() -> Result<PhysFrame, Error> {
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

    pub fn init(&mut self, boot_info: &'static BootInfo) {
        // We iterate in reverse order because big usable range are usually at the end.
        for region in boot_info.memory_map.iter().rev() {
            if region.region_type != MemoryRegionType::Usable {
                continue;
            }

            let range = region.range;

            if self.stack.len() == 0 {
                // We must init the stack, using part of this first free range.
                let total_frames = phys_view::physical_memory_size() as usize / PAGE_SIZE;
                let needed_frames =
                    align_up((total_frames * size_of::<u32>()) as u64, PAGE_SIZE as u64)
                        / PAGE_SIZE as u64;

                // Assert the range is big enough
                assert!(range.start_frame_number + needed_frames <= range.end_frame_number);

                let address = phys_view::to_virt_view(PhysAddr::new(range.start_addr()));

                self.stack =
                    unsafe { slice::from_raw_parts_mut(address.as_mut_ptr(), total_frames) };
                self.stack.fill(0);

                let start_frame_number = (range.start_frame_number + needed_frames) as u32;
                let end_frame_number = range.end_frame_number as u32;

                self.add_region(start_frame_number, end_frame_number);
            } else {
                let start_frame_number = range.start_frame_number as u32;
                let end_frame_number = range.end_frame_number as u32;

                self.add_region(start_frame_number, end_frame_number);
            }
        }

        println!(
            "Frame allocator: {} total frames, {} free",
            self.stack.len(),
            self.top
        );
    }

    fn add_region(&mut self, start_frame_number: u32, end_frame_number: u32) {
        for frame_number in start_frame_number..end_frame_number {
            assert!(self.top < self.stack.len());
            self.stack[self.top] = frame_number;
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
        (frame.start_address().as_u64() as usize / PAGE_SIZE) as u32
    }

    fn frame_number_to_frame(frame_number: u32) -> PhysFrame {
        let phys_addr = PhysAddr::new(frame_number as u64 * PAGE_SIZE as u64);
        PhysFrame::from_start_address(phys_addr).unwrap()
    }
}
