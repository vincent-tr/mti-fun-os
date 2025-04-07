// from https://github.com/rcore-os/buddy_system_allocator/

mod linked_list;

pub use self::linked_list::ListNode;
use crate::memory::PAGE_SIZE;
use core::{
    alloc::Layout,
    cmp::{max, min},
    mem::size_of,
    ptr,
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

pub trait NodeAllocator {
    /// Note: allocation cannot fail for now because we don't deal with failures in the middle of the algo
    unsafe fn allocate(&mut self) -> *mut ListNode;
    unsafe fn deallocate(&mut self, node: *mut ListNode);
}

/// buddy system with max order of `ORDER` and minimum request size of `UNIT_SIZE`
pub struct BuddyAllocator<const ORDER: usize, NodeAlloc: NodeAllocator> {
    free_list: [linked_list::LinkedList; ORDER],
    node_allocator: NodeAlloc,
    // Note: would like to use generic parameter but it's not possible
    unit_size: usize,

    // statistics
    stats: Stats,
}

unsafe impl<const ORDER: usize, NodeAlloc: NodeAllocator> Sync
    for BuddyAllocator<ORDER, NodeAlloc>
{
}

unsafe impl<const ORDER: usize, NodeAlloc: NodeAllocator> Send
    for BuddyAllocator<ORDER, NodeAlloc>
{
}

impl<const ORDER: usize, NodeAlloc: NodeAllocator> BuddyAllocator<ORDER, NodeAlloc> {
    /// Create an empty heap
    pub const fn new(node_allocator: NodeAlloc, unit_size: usize) -> Self {
        BuddyAllocator {
            free_list: [linked_list::LinkedList::new(); ORDER],
            node_allocator: node_allocator,
            unit_size: unit_size,
            stats: Stats {
                user: 0,
                allocated: 0,
                total: 0,
            },
        }
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

        while current_start < end {
            let lowbit = current_start & (!current_start + 1);
            let size = min(lowbit, prev_power_of_two(end - current_start));
            self.stats.total += size;

            let addr = VirtAddr::new_truncate(current_start as u64);

            unsafe {
                let new_node = self.alloc_node(addr);
                self.free_list[self.get_class(size)].push(new_node);
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

        let class = self.get_class(size);

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
                    let half_address = (*item).address() + self.get_size(cur_class - 1) as u64;
                    let split_item = self.alloc_node(half_address);
                    self.free_list[cur_class - 1].push(split_item);
                    self.free_list[cur_class - 1].push(item);
                }
            }

            let item = self.free_list[class]
                .pop()
                .expect("current block should have free space now");

            let address = unsafe { (*item).address() };
            unsafe {
                self.dealloc_node(item);
            }

            self.stats.user += layout.size();
            self.stats.allocated += size;

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
        let class = self.get_class(size);

        unsafe {
            // Put back into free list
            let item = self.alloc_node(ptr);
            self.free_list[class].push(item);

            // Merge free buddy lists
            let mut current_ptr = ptr;
            let mut current_class = class;
            while current_class < self.free_list.len() {
                let buddy = VirtAddr::new_truncate(
                    current_ptr.as_u64() ^ self.get_size(current_class) as u64,
                );
                let mut found = false;

                let mut prev_item: *mut ListNode = ptr::null_mut();

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
                let removed_node = self.free_list[current_class].remove_after(prev_item);
                self.dealloc_node(removed_node);

                let removed_node = self.free_list[current_class]
                    .pop()
                    .expect("item we just pushed not found");
                self.dealloc_node(removed_node);

                current_ptr = min(current_ptr, buddy);
                current_class += 1;

                let new_node = self.alloc_node(current_ptr);
                self.free_list[current_class].push(new_node);
            }
        }

        self.stats.user -= layout.size();
        self.stats.allocated -= size;
    }

    /// Return allocator stats
    pub fn stats(&self) -> Stats {
        self.stats.clone()
    }

    pub fn node_allocator(&mut self) -> &mut NodeAlloc {
        &mut self.node_allocator
    }

    fn get_size(&self, class: usize) -> usize {
        //const  UNIT_TRAILING_ZEROES: usize = UNIT_SIZE.trailing_zeros() as usize;
        // Note: cf. struct def
        return 1usize << (class + self.unit_size.trailing_zeros() as usize);
    }

    fn get_class(&self, size: usize) -> usize {
        //const  UNIT_TRAILING_ZEROES: usize = UNIT_SIZE.trailing_zeros() as usize;
        // Note: cf. struct def
        size.trailing_zeros() as usize - self.unit_size.trailing_zeros() as usize
    }

    unsafe fn alloc_node(&mut self, address: VirtAddr) -> *mut ListNode {
        let item = self.node_allocator.allocate();

        (*item).init(address);

        item
    }

    unsafe fn dealloc_node(&mut self, node: *mut ListNode) {
        self.node_allocator.deallocate(node);
    }
}

fn prev_power_of_two(num: usize) -> usize {
    1 << (usize::BITS as usize - num.leading_zeros() as usize - 1)
}
