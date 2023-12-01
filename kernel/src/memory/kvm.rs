use core::{alloc::Layout, mem};

use spin::RwLock;
use x86_64::{
    structures::paging::mapper::MapToError,
    VirtAddr,
};

use crate::memory::paging::Permissions;

use super::{
    buddy::{self, BuddyAllocator},
    paging, phys, PAGE_SIZE, VMALLOC_END, VMALLOC_START,
};

/*

4096 = 1 >> 12
4096 >> 17 = 512G

=> Buddy allocator for vm space
=> reserve 1G for kernel

*/

const BUDDY_ORDERS: usize = 16;

#[derive(Debug)]
pub enum AllocatorError {
    NoMemory,
    NoVirtualSpace,
}

static ALLOCATOR: RwLock<BuddyAllocator<BUDDY_ORDERS>> = RwLock::new(BuddyAllocator::new());

pub fn init() {
    let mut allocator = ALLOCATOR.write();
    allocator.set_area(VMALLOC_START, VMALLOC_END);
}

pub fn allocate(page_count: usize) -> Result<VirtAddr, AllocatorError> {
    assert!(page_count > 0);
    let layout = build_layout(page_count);

    let mut allocator = ALLOCATOR.write();
    let alloc_res = allocator.alloc(layout);
    if let Err(err) = alloc_res {
        // Ensure we matched all types
        match err {
            buddy::AllocatorError::NoMemory => {}
        }

        return Err(AllocatorError::NoVirtualSpace);
    }

    let addr = alloc_res.unwrap();

    match map_phys_alloc(addr, page_count) {
        Ok(_) => Ok(addr),
        Err(err) => {
            // remove address space reservation
            allocator.dealloc(addr, layout);

            Err(err)
        }
    }
}

pub fn deallocate(addr: VirtAddr, page_count: usize) {
    assert!(page_count > 0);
    assert!(addr.is_aligned(PAGE_SIZE as u64));
    assert!(addr >= VMALLOC_START);
    assert!(addr < VMALLOC_END);

    unmap_phys_alloc(addr, page_count);

    let mut allocator = ALLOCATOR.write();
    allocator.dealloc(addr, build_layout(page_count));
}

// Note: if part of the allocation fails, it is properly removed
fn map_phys_alloc(addr: VirtAddr, page_count: usize) -> Result<(), AllocatorError> {
    for page_index in 0..page_count {
        let frame_res = phys::allocate();

        if let Err(err) = frame_res {
            // Ensure we matched all types
            match err {
                phys::AllocatorError::NoMemory => {}
            }

            // Remove pages allocated so far
            if page_index > 0 {
                unmap_phys_alloc(addr, page_index);
            }

            return Err(AllocatorError::NoMemory);
        }

        let page_addr = addr + page_index * PAGE_SIZE;
        let mut frame = frame_res.unwrap();
        let perms = Permissions::READ | Permissions::WRITE;

        if let Err(err) = unsafe { paging::KERNEL_ADDRESS_SPACE.map(page_addr, &mut frame, perms) }
        {
            // Remove pages allocated so far
            if page_index > 0 {
                unmap_phys_alloc(addr, page_index);
            }

            match err {
                MapToError::FrameAllocationFailed => return Err(AllocatorError::NoMemory),
                MapToError::ParentEntryHugePage => {
                    panic!("Unexpected map error: ParentEntryHugePage")
                }
                MapToError::PageAlreadyMapped(_) => {
                    panic!("Unexpected map error: PageAlreadyMapped")
                }
            }
        }
    }

    Ok(())
}

fn unmap_phys_alloc(addr: VirtAddr, page_count: usize) {
    for page_index in 0..page_count {
        let page_addr = addr + page_index * PAGE_SIZE;
        let frame = unsafe {
            paging::KERNEL_ADDRESS_SPACE
                .unmap(addr)
                .expect("could not unmap page")
        };
        mem::drop(frame);
    }
}

fn build_layout(page_count: usize) -> Layout {
    unsafe { Layout::from_size_align_unchecked(page_count * PAGE_SIZE, PAGE_SIZE) }
}
