// from https://github.com/rcore-os/buddy_system_allocator/

mod linked_list;

use self::linked_list::ListNode;
use crate::memory::PAGE_SIZE;
use core::{
    alloc::{Allocator, Layout},
    cmp::{max, min},
    mem::size_of,
    ptr::{NonNull, null_mut},
};
use x86_64::VirtAddr;

#[derive(Debug)]
pub enum AllocatorError {
    NoMemory,
}

#[derive(Debug, Clone)]
pub struct Stats {
    /// The number of bytes that user requests
    pub user: usize,

    /// The number of bytes that are actually allocated
    pub allocated: usize,

    /// The total number of bytes in the heap
    pub total: usize,
}

pub struct BuddyAllocator<const ORDER: usize> {
    // buddy system with max order of `ORDER`
    free_list: [linked_list::LinkedList; ORDER],

    // statistics
    stats: Stats,
}

unsafe impl<const ORDER: usize> Sync for BuddyAllocator<ORDER> {}
unsafe impl<const ORDER: usize> Send for BuddyAllocator<ORDER> {}

impl<const ORDER: usize> BuddyAllocator<ORDER> {
    /// Create an empty heap
    pub const fn new() -> Self {
        BuddyAllocator {
            free_list: [linked_list::LinkedList::new(); ORDER],
            stats: Stats {
                user: 0,
                allocated: 0,
                total: 0,
            },
        }
    }

    /// Create an empty heap
    pub const fn empty() -> Self {
        Self::new()
    }

    /// Set the area where the heap will work.
    /// Will allocate `ListNode` objects for this.
    pub fn set_area(&mut self, begin: VirtAddr, end: VirtAddr) {
        assert!(self.stats.total == 0);
        assert!(begin.is_aligned(PAGE_SIZE as u64));
        assert!(end.is_aligned(PAGE_SIZE as u64));
        assert!(begin < end);

        let end = end.as_u64() as usize;
        let mut current_start = begin.as_u64() as usize;

        let mut total = 0;

        while current_start < end {
            let lowbit = current_start & (!current_start + 1);
            let size = min(lowbit, prev_power_of_two(end - current_start));
            total += size;

            let addr = VirtAddr::new_truncate(current_start as u64);

            unsafe {
                self.free_list[size.trailing_zeros() as usize].push(alloc_node(addr));
            }

            current_start += size;
        }
    }

    /// Alloc a range of memory from the heap satifying `layout` requirements
    pub fn alloc(&mut self, layout: Layout) -> Result<VirtAddr, AllocatorError> {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );

        let class = size.trailing_zeros() as usize;

        for cur_class in class..self.free_list.len() {
            // Find the first non-empty size class
            if self.free_list[cur_class].is_empty() {
                continue;
            }

            // Split buffers
            for cur_class in (class + 1..cur_class + 1).rev() {
                let item = self.free_list[cur_class]
                    .pop()
                    .expect("missed block to split");
                unsafe {
                    let split_item = alloc_node((*item).address() + (1u64 << (cur_class - 1)));
                    self.free_list[cur_class - 1].push(split_item);
                    self.free_list[cur_class - 1].push(item);
                }
            }

            let item = self.free_list[class]
                .pop()
                .expect("current block should have free space now");

            let address = unsafe { (*item).address() };
            unsafe {
                dealloc_node(item);
            }

            return Ok(address);
        }

        Err(AllocatorError::NoMemory)
    }

    /// Dealloc a range of memory from the heap
    pub fn dealloc(&mut self, ptr: VirtAddr, layout: Layout) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;

        unsafe {
            // Put back into free list
            let item = alloc_node(ptr);
            self.free_list[class].push(item);

            // Merge free buddy lists
            let mut current_ptr = ptr;
            let mut current_class = class;
            while current_class < self.free_list.len() {
                let buddy = VirtAddr::new_truncate(current_ptr.as_u64() ^ (1 << current_class));
                let mut found = false;

                let mut prev_item: *mut ListNode = null_mut();

                for item in self.free_list[current_class].iter() {
                    if (*item).address() == buddy {
                        found = true;
                        break;
                    }

                    prev_item = item;
                }

                if !found {
                    break;
                }

                // Free buddy found
                dealloc_node(self.free_list[current_class].remove_after(prev_item));
                dealloc_node(self.free_list[current_class].pop().expect("item we just pushed not found"));
                current_ptr = min(current_ptr, buddy);
                current_class += 1;
                self.free_list[current_class].push(alloc_node(current_ptr));
            }
        }

        self.stats.user -= layout.size();
        self.stats.allocated -= size;
    }

    /// Return allocator stats
    pub fn stats(&self) -> Stats {
        self.stats.clone()
    }
}

fn prev_power_of_two(num: usize) -> usize {
    1 << (usize::BITS as usize - num.leading_zeros() as usize - 1)
}

unsafe fn alloc_node(address: VirtAddr) -> *mut ListNode {
    let allocator = alloc::alloc::Global;
    let layout = Layout::new::<ListNode>();
    let item = allocator
        .allocate(layout)
        .expect("allocation failed.")
        .as_mut_ptr() as *mut ListNode;

    (*item).init(address);

    item
}

unsafe fn dealloc_node(node: *mut ListNode) {
    let allocator = alloc::alloc::Global;
    let layout = Layout::new::<ListNode>();
    allocator.deallocate(NonNull::new_unchecked(node as *mut u8), layout);
}
