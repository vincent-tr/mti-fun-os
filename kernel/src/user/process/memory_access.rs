use core::{
    mem::{align_of, size_of},
    ops::Range, slice::{from_raw_parts, from_raw_parts_mut},
};

use alloc::vec::Vec;

use crate::{
    memory::{
        self, page_aligned_down, page_aligned_up, AddressSpace, FrameRef, Permissions, VirtAddr,
        PAGE_SIZE,
    },
    user::{
        error::{check_permissions, out_of_memory},
        Error,
    },
};

/// Represent a memory access to some of the process VM space.
pub struct MemoryAccess {
    alloc: VirtAddr,
    pages: usize,
    base: VirtAddr,
    size: usize,
}

impl MemoryAccess {
    fn create(
        address_space: &AddressSpace,
        range: Range<VirtAddr>,
        perms: Permissions,
    ) -> Result<Self, Error> {
        assert!(perms != Permissions::NONE);

        let process_range = VirtAddr::new(page_aligned_down(range.start.as_u64() as usize) as u64)
            ..VirtAddr::new(page_aligned_up(range.end.as_u64() as usize) as u64);
        let range_offset = range.start - process_range.start;

        let mut frames = Vec::new();

        for process_addr in process_range.step_by(PAGE_SIZE) {
            let (phys_addr, actual_perms) = unsafe { address_space.get_infos(process_addr) };
            check_permissions(actual_perms, perms)?;

            frames
                .push(unsafe { FrameRef::unborrow(phys_addr.expect("Unexpected missing frame")) });
        }

        let alloc = if let Some(alloc) = unsafe { memory::map_phys(&frames) } {
            alloc
        } else {
            return Err(out_of_memory());
        };


        Ok(Self {
            alloc,
            pages: frames.len(),
            base: alloc + range_offset,
            size: (range.end - range.start) as usize,
        })
    }

    pub fn get<'a, T>(&'a self) -> &'a T {
        self.assert_layout::<T>();

        unsafe { &*(self.base.as_ptr()) }
    }

    pub fn get_mut<'a, T>(&'a mut self) -> &'a mut T {
        self.assert_layout::<T>();

        unsafe { &mut *(self.base.as_mut_ptr()) }
    }

    pub fn get_slice<'a, T>(&'a self) -> &'a [T] {
        self.assert_layout_slice::<T>();

        let ptr = self.base.as_ptr::<T>();
        unsafe { from_raw_parts(ptr, self.size / size_of::<T>()) }
    }

    pub fn get_slice_mut<'a, T>(&'a mut self) -> &'a mut [T] {
        self.assert_layout_slice::<T>();

        let ptr = self.base.as_mut_ptr::<T>();
        unsafe { from_raw_parts_mut(ptr, self.size / size_of::<T>()) }
    }

    fn assert_layout<T>(&self) {
        assert!(size_of::<T>() <= self.size);

        let addr = self.base.as_u64() as usize;
        // Alignment is always a power of 2, so we can use bit ops instead of a mod here.
        assert!((addr & (align_of::<T>() - 1)) == 0);
    }

    fn assert_layout_slice<T>(&self) {
        assert!(self.size % size_of::<T>() == 0);

        let addr = self.base.as_u64() as usize;
        // Alignment is always a power of 2, so we can use bit ops instead of a mod here.
        assert!((addr & (align_of::<T>() - 1)) == 0);
    }
}

impl Drop for MemoryAccess {
    fn drop(&mut self) {
        memory::unmap_phys(self.alloc, self.pages);
    }
}

/// Standalone function, so that MemoryAccess::create() can remain private
/// 
/// Create a new memory access
///
/// permissions are the at least excepted permission in address space.
///
/// eg: if READ is set, then the range must be mapped in the address space with at least READ permission
pub fn create(
    address_space: &AddressSpace,
    range: Range<VirtAddr>,
    perms: Permissions,
) -> Result<MemoryAccess, Error> {
    MemoryAccess::create(address_space, range, perms)
}
