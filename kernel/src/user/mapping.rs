use crate::memory::{is_page_aligned, is_userspace, MapError, UnmapError, Permissions, VirtAddr, PAGE_SIZE};
use alloc::sync::{Arc, Weak};

use super::{process::Process, MemoryObject};

pub struct Mapping {
    process: Weak<Process>,
    addr: VirtAddr,
    size: usize,
    memory_object: Arc<MemoryObject>,
    offset: usize,
}

/// Mapping of a memory object in a process
impl Mapping {
    /// Create a new mapping
    ///
    /// # Safety
    ///
    /// - addr must point to userspace address
    /// - addr must be free in the process address space
    /// - offset + size must be <= memory_object.size()
    /// - addr, size, offset must be aligned
    pub unsafe fn new(
        process: &Arc<Process>,
        addr: VirtAddr,
        size: usize,
        perms: Permissions,
        memory_object: Arc<MemoryObject>,
        offset: usize,
    ) -> Self {
        assert!(is_userspace(addr));
        assert!(size + offset <= memory_object.size());
        assert!(is_page_aligned(addr.as_u64() as usize));
        assert!(is_page_aligned(size));
        assert!(is_page_aligned(offset));

        let mut phys_offset = offset;
        let mut virt_addr = addr;
        let mut done_size = 0;

        let address_space = process.address_space_mut();

        while done_size < size {
            let frame = &mut memory_object.frame(phys_offset).clone();

            match address_space.map(virt_addr, frame, perms) {
                Ok(_) => {}
                Err(err) => {
                    // match all arms
                    match err {
                        // TODO: should handle FrameAllocationFailed
                        MapError::FrameAllocationFailed => panic!("Unexpected error FrameAllocationFailed"),
                        MapError::ParentEntryHugePage => panic!("Unexpected error ParentEntryHugePage"),
                        MapError::PageAlreadyMapped(_) => panic!("Unexpected error PageAlreadyMapped"),
                    }
                }
            }

            phys_offset += PAGE_SIZE;
            virt_addr += PAGE_SIZE;
            done_size += PAGE_SIZE;
        }

        Mapping {
            process: Arc::downgrade(process),
            addr,
            size,
            memory_object,
            offset,
        }
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
        let process = self.process();

        let (_, perm) = unsafe { process.address_space().get_infos(self.addr) };

        perm
    }

    /// Get the memory object this mapping is pointing to
    pub fn memory_object(&self) -> &MemoryObject {
        &self.memory_object
    }

    /// Get the offset in the memory object at which this mapping starts
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
      let mut process = self.process();

      let mut virt_addr = self.addr;
      let mut done_size = 0;

      let address_space = process.address_space_mut();

      while done_size < self.size {
          match unsafe {address_space.unmap(virt_addr) } {
              Ok(_) => {}
              Err(err) => {
                  // match all arms
                  match err {
                    UnmapError::ParentEntryHugePage => panic!("Unexpected error ParentEntryHugePage"),
                    UnmapError::PageNotMapped => panic!("Unexpected error PageNotMapped"),
                    UnmapError::InvalidFrameAddress(_) => panic!("Unexpected error InvalidFrameAddress"),
                }
              }
          }

          virt_addr += PAGE_SIZE;
          done_size += PAGE_SIZE;
      }

    }
}
