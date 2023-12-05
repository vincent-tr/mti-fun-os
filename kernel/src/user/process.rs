use alloc::{
    collections::LinkedList,
    sync::{Arc, Weak},
};

use crate::{
    memory::{
        is_page_aligned, is_userspace, AddressSpace, MapError, Permissions, UnmapError, VirtAddr,
        PAGE_SIZE,
    },
    user::error::out_of_memory,
};

use super::{
    error::{check_arg, check_is_userspace, check_page_alignment},
    Error, MemoryObject,
};

/// Process
pub struct Process {
    address_space: AddressSpace,
    mappings: LinkedList<Mapping>,
}

impl Process {}

pub struct Mapping {
    process: Weak<Process>,
    addr: VirtAddr,
    size: usize,
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
        memory_object: Option<Arc<MemoryObject>>, // may be null if perms is NONE
        offset: usize,
    ) -> Result<Self, Error> {
        check_is_userspace(addr)?;
        check_page_alignment(addr.as_u64() as usize)?;
        check_page_alignment(size)?;
        check_page_alignment(offset)?;

        if let Some(mobj) = memory_object {
            // Force some access on memory object, this ease checks
            check_arg(perms != Permissions::NONE)?;
            check_arg(size + offset <= mobj.size())?;
        } else {
            check_arg(perms == Permissions::NONE)?;
        }

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
        assert!(addr < self.addr + self.size);

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
    /// - the other mapping have to start at the end of self.
    /// - both mapping permissions must be same
    /// - if they are referencing a MemoryObject, it must be the same, and offset must correspond
    pub unsafe fn merge(&mut self, mut other: Mapping) {
        assert!(other.addr == self.addr + self.size);
        assert!(other.permissions() == self.permissions());

        if let Some(&lower_mobj) = self.memory_object.as_ref() {
            assert!(Arc::ptr_eq(&lower_mobj, &other.memory_object.unwrap()));
            assert!(other.offset == self.offset + self.size);
        }

        self.size += other.size;
        other.size = 0; // Do not drop mapping on leave
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
