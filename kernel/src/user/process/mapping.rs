
use core::ops::Range;

use alloc::sync::{Arc, Weak};

use crate::{memory::{
    is_page_aligned, is_userspace, MapError,
    Permissions, UnmapError, VirtAddr, PAGE_SIZE,
}, user::{MemoryObject, Error, error::out_of_memory}};

use super::Process;

pub struct Mapping {
    process: Weak<Process>,
    range: Range<VirtAddr>,
    /// null if perms is NONE
    memory_object: Option<Arc<MemoryObject>>,
    offset: usize,
}

/// Mapping of a memory object in a process
impl Mapping {
    /// Create a new mapping
    pub fn new(
        process: &Arc<Process>,
        range: Range<VirtAddr>,
        perms: Permissions,
        memory_object: Option<Arc<MemoryObject>>,
        offset: usize,
    ) -> Result<Self, Error> {
        let mut mapping = Mapping {
            process: Arc::downgrade(process),
            range,
            memory_object,
            offset,
        };

        if let Some(ref _mobj) = mapping.memory_object {
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

    /// Get the range of this mapping
    pub fn range(&self) -> &Range<VirtAddr> {
        &self.range
    }

    /// Get the size of this mapping
    pub fn size(&self) -> usize {
        (self.range.end - self.range.start) as usize
    }

    /// Get the permissions of the mapping
    pub fn permissions(&self) -> Permissions {
        let process = self.process();
        let address_space = process.address_space().read();

        let (_, perm) = unsafe { address_space.get_infos(self.range.start) };

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
        let range = self.range().clone();
        assert!(addr > range.start);
        assert!(addr < range.end);

        self.range = range.start..addr;

        let other_offset = if let Some(_) = self.memory_object {
            self.offset + self.size()
        } else {
            0
        };

        Mapping {
            process: self.process.clone(),
            range: addr..range.end,
            memory_object: self.memory_object.clone(),
            offset: other_offset,
        }
    }

    /// Merge another mapping at the end of this one.
    ///
    /// # Safety
    /// can_merge() must be true
    pub unsafe fn merge(&mut self, mut other: Mapping) {
        self.range = self.range.start..other.range.end;
        other.range = other.range.end..other.range.end; // Do not drop mapping on leave
    }

    /// Test if the other mapping camn be merged into self:
    /// - the other mapping have to start at the end of self.
    /// - both mapping permissions must be same
    /// - if they are referencing a MemoryObject, it must be the same, and offset must correspond
    pub fn can_merge(&self, other: &Mapping) -> bool {
        if self.range().end != other.range().start || other.permissions() != self.permissions() {
            return false;
        }

        if let Some(lower_mobj) = self.memory_object.as_ref() {
            if !(Arc::ptr_eq(&lower_mobj, other.memory_object.as_ref().unwrap()))
                || other.offset != self.offset + self.size()
            {
                return false;
            }
        }

        return true;
    }

    unsafe fn map(&mut self, perms: Permissions) -> Result<(), Error> {
        let mut phys_offset = self.offset;
        let mut virt_addr = self.range.start;

        let process = self.process();
        let mut address_space = process.address_space().write();
        let mobj = self.memory_object.as_ref().unwrap();

        while virt_addr < self.range.end {
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
                            self.range = self.range.start..virt_addr;
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
        }

        Ok(())
    }

    unsafe fn unmap(&mut self) {
        let process = self.process();
        let mut address_space = process.address_space().write();

        let mut virt_addr = self.range.start;

        while virt_addr < self.range.end {
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
