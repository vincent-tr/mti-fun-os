use core::ops::{Bound, Range};
use hashbrown::HashMap;
use rangemap::RangeMap;
use lazy_static::lazy_static;

use alloc::{
    collections::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
};
use spin::RwLock;

use crate::memory::{
    create_adress_space, is_page_aligned, is_userspace, AddressSpace, AllocatorError, MapError,
    Permissions, UnmapError, VirtAddr, KERNEL_START, PAGE_SIZE,
};

use super::mapping::Mapping;

use crate::user::{
    error::{check_arg, check_is_userspace, check_page_alignment, check_positive, out_of_memory},
    Error, MemoryObject,
};

/// Standalone function, so that Process::new() can remain private
/// 
/// Note: Only Process type is exported by process module, not this function
pub fn new(id: u32) -> Result<Arc<Process>, Error> {
    Process::new(id)
}

const USER_SPACE: Range<VirtAddr> = (VirtAddr::zero() + PAGE_SIZE)..KERNEL_START;

/// Process
pub struct Process {
    id: u32,
    address_space: RwLock<AddressSpace>,
    /// Note: ordered by address
    mappings: RwLock<RangeMap<VirtAddr, Mapping>>,
}

impl Process {
    fn new(id: u32) -> Result<Arc<Self>, Error> {
        let address_space = match create_adress_space() {
            Ok(address_space) => address_space,
            Err(err) => {
                // match all arms
                match err {
                    AllocatorError::NoMemory => Err(out_of_memory())?,
                }
            }
        };

        Ok(Arc::new(Self {
            id,
            address_space: RwLock::new(address_space),
            mappings: RwLock::new(RangeMap::new()),
        }))
    }

    /// Get the process identifier
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get address space of the process
    pub fn address_space(&self) -> &RwLock<AddressSpace> {
      &self.address_space
    }

    /// Map a MemoryObject (or part of it) into the process address space, with the given permissions.
    ///
    /// Notes:
    /// - If `addr` is `null`, an address where the mapping can fit will be found.
    /// - If `addr` is not `null`, this function cannot overwrite part of an existing mapping. Call unmap() before.
    ///
    pub fn map(
        self: &Arc<Self>,
        mut addr: VirtAddr,
        size: usize,
        perms: Permissions,
        memory_object: Option<Arc<MemoryObject>>,
        offset: usize,
    ) -> Result<VirtAddr, Error> {
        if !addr.is_null() {
            check_is_userspace(addr)?;
            check_page_alignment(addr.as_u64() as usize)?;
            check_is_userspace(addr + size)?;
        }
        check_positive(size);
        check_page_alignment(size)?;
        check_page_alignment(offset)?;

        if let Some(ref mobj) = memory_object {
            // Force some access on memory object, this ease checks
            check_arg(perms != Permissions::NONE)?;
            check_arg(size + offset <= mobj.size())?;
        } else {
            check_arg(perms == Permissions::NONE)?;
        }

        let mut mappings = self.mappings.write();

        // Other checks are done in Mapping::new().
        if addr.is_null() {
            addr = Self::find_space(&mappings, size)?;
        }

        let mut mapping = Mapping::new(self, addr, size, perms, memory_object, offset)?;

        // Check if we can merge with prev/next item
        if let Some(next) = mappings.upper_bound(Bound::Excluded(&addr)).value()
            && mapping.can_merge(next)
        {
            let next_addr = next.address();
            let to_merge = mappings
                .remove(&next_addr)
                .expect("mappings access mismatch");
            unsafe { mapping.merge(to_merge) };
        }

        if let Some(prev) = mappings.lower_bound_mut(Bound::Excluded(&addr)).value_mut()
            && prev.can_merge(&mapping)
        {
            unsafe { prev.merge(mapping) };
        } else {
            assert!(
                mappings.insert(addr, mapping).is_none(),
                "mappings key overwrite"
            );
        }

        Ok(addr)
    }

    fn find_space(mappings: &RangeMap<VirtAddr, Mapping>, size: usize) -> Result<VirtAddr, Error> {
      mappings.gaps(USER_SPACE);

        let mut prev_end = VirtAddr::zero() + PAGE_SIZE; // Do not put mapping at address 0

        // First fit
        for (addr, mapping) in mappings {
            if mapping.address() > prev_end && ((mapping.address() - prev_end) as usize) < size {
                return Ok(prev_end);
            }

            prev_end = mapping.end();
        }

        // Is there room at the end?
        if ((KERNEL_START - prev_end) as usize) < size {
            return Ok(prev_end);
        }

        Err(out_of_memory())
    }

    /// Unmap the address space from addr to addr+size.
    /// Notes:
    /// - It may contains multiple mappings,
    /// - addr or addr+size may be in the middle of a mapping
    /// - part of the specified area my not be mapped. In consequence, calling unmap() on an unmapped area is a successful noop.
    ///
    pub fn unmap(&mut self, addr: VirtAddr, size: usize) {
        let end = addr + size;
        let mut mappings = self.mappings.write();

        if mappings.len() == 0 {
            return;
        } else if let Some((_, first_mapping)) = mappings.first_key_value()
            && end < first_mapping.address()
        {
            return;
        } else if let Some((_, last_mapping)) = mappings.last_key_value()
            && addr > last_mapping.end()
        {
            return;
        }

        let mut cursor = mappings.lower_bound_mut(Bound::Included(&addr));
        // we checked that the given range may cross items
        assert!(cursor.key().is_some());
        // Move to the prev item if possible: addr may start in the middle of the previous mapping
        if let Some((_, mapping)) = cursor.peek_prev()
            && mapping.contains(addr)
        {
            cursor.move_prev();

            // Need to split: not on a boundary
            let new_mapping = mapping.split(addr);
            cursor.insert_after(new_mapping.address(), new_mapping);

            cursor.move_next();
        }

        // Remove full mappings
        while let Some((_, mapping)) = cursor.key_value()
            && mapping.end() <= end
        {
            cursor.remove_current().expect("unexpected empty mapping");
        }

        // Last mapping: may need split
        cursor.move_next();
        if let Some((_, mapping)) = cursor.key_value()
            && mapping.address() < end
        {
            let new_mapping = mapping.split(end);
            cursor.insert_after(new_mapping.address(), new_mapping);

            cursor.remove_current().expect("unexpected empty mapping");
        }
    }
}
