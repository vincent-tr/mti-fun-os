use core::ops::Bound;

use alloc::sync::Arc;
use spin::RwLock;

use crate::memory::{create_adress_space, AddressSpace, AllocatorError, Permissions, VirtAddr};

use super::{mapping::Mapping, mappings::Mappings};

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

/// Process
pub struct Process {
    id: u32,
    address_space: RwLock<AddressSpace>,
    /// Note: ordered by address
    mappings: RwLock<Mappings>,
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
            mappings: RwLock::new(Mappings::new()),
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
        addr: VirtAddr,
        size: usize,
        perms: Permissions,
        memory_object: Option<Arc<MemoryObject>>,
        offset: usize,
    ) -> Result<VirtAddr, Error> {
        check_positive(size)?;
        check_page_alignment(size)?;
        check_page_alignment(offset)?;

        if !addr.is_null() {
            check_is_userspace(addr)?;
            check_page_alignment(addr.as_u64() as usize)?;
            check_is_userspace(addr + size)?;
        }

        if let Some(ref mobj) = memory_object {
            // Force some access on memory object, this ease checks
            check_arg(perms != Permissions::NONE)?;
            check_arg(size + offset <= mobj.size())?;
        } else {
            check_arg(perms == Permissions::NONE)?;
        }

        // Other checks are done in Mapping::new().

        let mut mappings = self.mappings.write();

        let range = if addr.is_null() {
            mappings.find_space(size)?
        } else {
            let range = addr..addr + size;
            check_arg(!mappings.overlaps(&range))?;
            range
        };

        let mapping = Mapping::new(self, range, perms, memory_object, offset)?;

        mappings.add(mapping);

        Ok(addr)
    }

    /// Unmap the address space from addr to addr+size.
    /// Notes:
    /// - It may contains multiple mappings,
    /// - addr or addr+size may be in the middle of a mapping
    /// - part of the specified area my not be mapped. In consequence, calling unmap() on an unmapped area is a successful noop.
    ///
    pub fn unmap(&mut self, addr: VirtAddr, size: usize) -> Result<(), Error> {
        check_positive(size);
        check_page_alignment(size)?;
        check_is_userspace(addr)?;
        check_page_alignment(addr.as_u64() as usize)?;
        check_is_userspace(addr + size)?;

        let mut mappings = self.mappings.write();

        mappings.remove_range(addr..addr+size);
        
        Ok(())
/*
        for area in 

        let mut cur_addr = addr;
        let mut cur_size = size;

        loop {
            mappings.overlapping(range)
        }

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
        */
    }
}
