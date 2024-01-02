use core::{
    marker::PhantomData,
    mem::{align_of, size_of},
    ops::Range,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use alloc::vec::Vec;

use crate::{
    memory::{
        self, page_aligned_down, page_aligned_up, AddressSpace, FrameRef, Permissions, PhysAddr,
        VirtAddr, PAGE_SIZE,
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
            let phys_addr = phys_addr.expect("Unexpected missing frame");

            // Get a new ref from the phys addr
            let frame = unsafe { ref_frame(phys_addr) };

            frames.push(frame);
        }

        let alloc = if let Some(alloc) = unsafe { memory::map_phys(&mut frames) } {
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

/// Get a frame ref directly from a physical address
///
/// # Safety
///
/// The physical address must have been borrowed from a FrameRef earlier
///
unsafe fn ref_frame(phys_addr: PhysAddr) -> FrameRef {
    // Temp unborrow the frame from phys addr
    let mut borrowed_frame = FrameRef::unborrow(phys_addr);

    // Add new ref
    let frame_new_ref = borrowed_frame.clone();

    // Borrow it back
    borrowed_frame.borrow();

    frame_new_ref
}

/// Represent a memory access to some of the process VM space.
pub struct TypedMemoryAccess<T> {
    access: MemoryAccess,
    _phantom: PhantomData<T>,
}

impl<T> TypedMemoryAccess<T> {
    pub fn get<'a>(&'a self) -> &'a T {
        self.access.get()
    }

    pub fn get_mut<'a>(&'a mut self) -> &'a mut T {
        self.access.get_mut()
    }
}

/// Standalone function, so that MemoryAccess::create() can remain private
///
/// Create a new memory access
///
/// permissions are the at least excepted permission in address space.
///
/// eg: if READ is set, then the range must be mapped in the address space with at least READ permission
pub fn create_typed<T>(
    address_space: &AddressSpace,
    addr: VirtAddr,
    perms: Permissions,
) -> Result<TypedMemoryAccess<T>, Error> {
    let range = addr..addr + size_of::<T>();
    let access = MemoryAccess::create(address_space, range, perms)?;

    Ok(TypedMemoryAccess {
        access,
        _phantom: PhantomData,
    })
}

/// Represent a memory access to some of the process VM space.
pub struct TypedSliceMemoryAccess<T> {
    access: MemoryAccess,
    _phantom: PhantomData<T>,
}

impl<T> TypedSliceMemoryAccess<T> {
    pub fn get<'a>(&'a self) -> &'a [T] {
        self.access.get_slice()
    }

    pub fn get_mut<'a>(&'a mut self) -> &'a mut [T] {
        self.access.get_slice_mut()
    }
}

/// Standalone function, so that MemoryAccess::create() can remain private
///
/// Create a new memory access
///
/// permissions are the at least excepted permission in address space.
///
/// eg: if READ is set, then the range must be mapped in the address space with at least READ permission
pub fn create_typed_slice<T>(
    address_space: &AddressSpace,
    addr: VirtAddr,
    count: usize,
    perms: Permissions,
) -> Result<TypedSliceMemoryAccess<T>, Error> {
    let range = addr..addr + (count * size_of::<T>());
    let access = MemoryAccess::create(address_space, range, perms)?;

    Ok(TypedSliceMemoryAccess {
        access,
        _phantom: PhantomData,
    })
}
