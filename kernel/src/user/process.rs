use core::ops::Bound;
use hashbrown::HashMap;

use alloc::{
    collections::BTreeMap,
    sync::{Arc, Weak}, vec::Vec,
};
use spin::RwLock;

use crate::memory::{
    is_page_aligned, is_userspace, AddressSpace, MapError, Permissions, UnmapError, VirtAddr,
    KERNEL_START, PAGE_SIZE,
};

use super::{
    error::{check_arg, check_is_userspace, check_page_alignment, check_positive, out_of_memory},
    Error, MemoryObject, id_gen::IdGen,
};

pub struct Processes {
    id_gen: IdGen,
    processes: RwLock<HashMap<u32, Weak<Process>>>
}

impl Processes {
    const fn new() -> Self {
        Self { 
            id_gen: IdGen::new(),
            processes: RwLock::new(HashMap::new())
        }
    }

    /// Create a new process
    pub fn create(&mut self) -> Arc<Process> {
        self.clean_map();
        
        let id = self.id_gen.generate();
        let process = Process::new(id);

        let mut map = self.processes.write();
        assert!(map.insert(id, Arc::downgrade(&process)).is_none(), "unepxected map overwrite");

        process
    }

    /// Find a process by its pid
    pub fn find(&self, pid: u32) -> Option<Arc<Process>> {
        self.clean_map();

        let map = self.processes.read();
        if let Some(weak) = map.get(&pid) {
            return weak.upgrade();
        } else {
            None
        }
    }

    fn clean_map(&self) {
        let map = self.processes.upgradeable_read();

        let mut delete_list = Vec::new();

        for (pid, weak) in map.iter() {
            if weak.strong_count() == 0 {
                delete_list.push(pid);
            }
        }

        if delete_list.len() > 0 {
            let mut map = map.upgrade();
            for pid in delete_list {
                map.remove(pid);
            }
        }
    }
}

pub static PROCESSES: Processes = Processes::new();

/// Process
pub struct Process {
    id: u32,
    address_space: AddressSpace,
    /// Note: ordered by address
    mappings: RwLock<BTreeMap<VirtAddr, Mapping>>,
}

unsafe impl Sync for Process {}
unsafe impl Send for Process {}

impl Process {
    const fn new(id: u32) -> Arc<Self> {
        Arc::new(Self {
            id,
            address_space: todo!(),
            mappings: RwLock::new(BTreeMap::new()),
        })
    }

    /// Get the process identifier
    pub fn id(&self) -> u32 {
        self.id
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

        if let Some(mobj) = memory_object {
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
            let to_merge = mappings
                .remove(&next.address())
                .expect("mappings access mismatch");
            unsafe { mapping.merge(to_merge) };
        }

        if let Some(prev) = mappings.lower_bound(Bound::Excluded(&addr)).value()
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

    fn find_space(mappings: &BTreeMap<VirtAddr, Mapping>, size: usize) -> Result<VirtAddr, Error> {
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

struct Mapping {
    process: Weak<Process>,
    addr: VirtAddr,
    size: usize,
    /// null if perms is NONE
    memory_object: Option<Arc<MemoryObject>>,
    offset: usize,
}

/// Mapping of a memory object in a process
impl Mapping {
    /// Create a new mapping
    pub fn new(
        process: &Arc<Process>,
        addr: VirtAddr,
        size: usize,
        perms: Permissions,
        memory_object: Option<Arc<MemoryObject>>,
        offset: usize,
    ) -> Result<Self, Error> {
        let mut mapping = Mapping {
            process: Arc::downgrade(process),
            addr,
            size,
            memory_object,
            offset,
        };

        if let Some(_) = memory_object {
            unsafe {
                // If the map fails, size has been sert to the partially mapped part, so that the mapping is consistent.
                // Leaving will drop the partial map properly.
                mapping.map(perms)?;
            }
        }

        Ok(mapping)
    }

    /// Get the process this mapping is rattached
    pub fn process(&self) -> Arc<Process> {
        self.process
            .upgrade()
            .expect("Could not get Mapping's process")
    }

    /// Get the address of the start of the mapping
    pub fn address(&self) -> VirtAddr {
        self.addr
    }

    /// Get the size in bytes of the mapping
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the exclusive end address of the mapping
    pub fn end(&self) -> VirtAddr {
        self.addr + self.size
    }

    /// Indicate if the given address is inside the mapping
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.addr && addr < self.end()
    }

    /// Indicate if the current mapping overlap the given range
    pub fn intersect(&self, addr: VirtAddr, size: usize) -> bool {
        addr >= self.addr && addr < self.end()
    }

    /// Get the permissions of the mapping
    pub fn permissions(&self) -> Permissions {
        let mut process = self.process();

        let (_, perm) = unsafe { process.address_space.get_infos(self.addr) };

        perm
    }

    pub fn set_permissions(&mut self, perms: Permissions) -> Result<(), Error> {
        todo!();
    }

    /// Get the memory object this mapping is pointing to
    pub fn memory_object(&self) -> Option<&Arc<MemoryObject>> {
        self.memory_object.as_ref()
    }

    /// Get the offset in the memory object at which this mapping starts
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Split this mapping at `addr` into 2 parts.
    ///
    /// self will have the lower part, and the return value will have the higher part.
    ///
    /// Both will have same MemoryObject, and same permissions
    pub fn split(&mut self, addr: VirtAddr) -> Mapping {
        assert!(is_userspace(addr));
        assert!(is_page_aligned(addr.as_u64() as usize));

        // Do not allow mapping of size == 0
        assert!(addr > self.addr);
        assert!(addr < self.end());

        let new_size = (addr - self.addr) as usize;
        let other_size = self.size - new_size;
        self.size = new_size;

        let other_offset = if let Some(_) = self.memory_object {
            self.offset + self.size
        } else {
            0
        };

        Mapping {
            process: self.process,
            addr: addr,
            size: other_size,
            memory_object: self.memory_object,
            offset: other_offset,
        }
    }

    /// Merge another mapping at the end of this one.
    ///
    /// # Safety
    /// can_merge() must be true
    pub unsafe fn merge(&mut self, mut other: Mapping) {
        self.size += other.size;
        other.size = 0; // Do not drop mapping on leave
    }

    /// Test if the other mapping camn be merged into self:
    /// - the other mapping have to start at the end of self.
    /// - both mapping permissions must be same
    /// - if they are referencing a MemoryObject, it must be the same, and offset must correspond
    fn can_merge(&self, other: &Mapping) -> bool {
        if other.addr != self.end() || other.permissions() != self.permissions() {
            return false;
        }

        if let Some(&lower_mobj) = self.memory_object.as_ref() {
            if !(Arc::ptr_eq(&lower_mobj, &other.memory_object.unwrap()))
                || other.offset != self.offset + self.size
            {
                return false;
            }
        }

        return true;
    }

    unsafe fn map(&mut self, perms: Permissions) -> Result<(), Error> {
        let mut phys_offset = self.offset;
        let mut virt_addr = self.addr;
        let mut done_size = 0;

        let address_space = &mut self.process().address_space;
        let mobj = self.memory_object.unwrap();

        while done_size < self.size {
            let mut frame = mobj.frame(phys_offset).clone();

            match address_space.map(virt_addr, &mut frame, perms) {
                Ok(_) => {}
                Err(err) => {
                    // match all arms
                    match err {
                        MapError::FrameAllocationFailed => {
                            // Mapping failed.
                            // We update the size to the currently done size.
                            // So the mapping is valid even if incomplete, and we can drop it properly (and unmap)
                            self.size = done_size;
                            return Err(out_of_memory());
                        }
                        MapError::ParentEntryHugePage => {
                            panic!("Unexpected error ParentEntryHugePage")
                        }
                        MapError::PageAlreadyMapped(_) => {
                            panic!("Unexpected error PageAlreadyMapped")
                        }
                    }
                }
            }

            phys_offset += PAGE_SIZE;
            virt_addr += PAGE_SIZE;
            done_size += PAGE_SIZE;
        }

        Ok(())
    }

    unsafe fn unmap(&mut self) {
        let mut process = self.process();

        let mut virt_addr = self.addr;
        let mut done_size = 0;

        let address_space = &mut process.address_space;

        while done_size < self.size {
            match address_space.unmap(virt_addr) {
                Ok(_) => {}
                Err(err) => {
                    // match all arms
                    match err {
                        UnmapError::ParentEntryHugePage => {
                            panic!("Unexpected error ParentEntryHugePage")
                        }
                        UnmapError::PageNotMapped => panic!("Unexpected error PageNotMapped"),
                        UnmapError::InvalidFrameAddress(_) => {
                            panic!("Unexpected error InvalidFrameAddress")
                        }
                    }
                }
            }

            virt_addr += PAGE_SIZE;
            done_size += PAGE_SIZE;
        }
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        if let Some(_) = self.memory_object {
            unsafe {
                self.unmap();
            }
        }
    }
}
