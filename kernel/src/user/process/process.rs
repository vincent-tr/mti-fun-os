use core::ops::Range;

use alloc::sync::Arc;
use log::debug;
use spin::RwLock;

use crate::memory::{create_adress_space, AddressSpace, AllocatorError, Permissions, VirtAddr};

use super::{mapping::Mapping, mappings::Mappings, memory_access, MemoryAccess};

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
#[derive(Debug)]
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

        let process = Arc::new(Self {
            id,
            address_space: RwLock::new(address_space),
            mappings: RwLock::new(Mappings::new()),
        });

        debug!("Process {} created", process.id);

        Ok(process)
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

        let mapping = Mapping::new(self, range.clone(), perms, memory_object, offset)?;
        let addr = mapping.range().start;

        mappings.add(mapping);

        debug!(
            "Process {}: mapped at {:?} with perms {:?}",
            self.id, range, perms
        );

        Ok(addr)
    }

    /// Unmap the address space from addr to addr+size.
    /// Notes:
    /// - It may contains multiple mappings,
    /// - addr or addr+size may be in the middle of a mapping
    /// - part of the specified area my not be mapped. In consequence, calling unmap() on an unmapped area is a successful noop.
    ///
    pub fn unmap(self: &Arc<Self>, addr: VirtAddr, size: usize) -> Result<(), Error> {
        check_positive(size)?;
        check_page_alignment(size)?;
        check_is_userspace(addr)?;
        check_page_alignment(addr.as_u64() as usize)?;
        check_is_userspace(addr + size)?;

        let mut mappings = self.mappings.write();

        let range = addr..addr + size;

        mappings.remove_range(range.clone());

        debug!("Process {}: unmapped at {:?}", self.id, range);

        Ok(())
    }

    /// Create a new memory access to a part of the process VM
    ///
    ///
    /// permissions are the at least excepted permission in address space.
    ///
    /// eg: if READ is set, then the range must be mapped in the address space with at least READ permission
    /// 
    /// Note that the kernel access itself it always READ/WRITE
    pub fn vm_access(
        self: &Arc<Self>,
        range: Range<VirtAddr>,
        perms: Permissions,
    ) -> Result<MemoryAccess, Error> {
        let address_space = self.address_space().read();
        memory_access::create(&address_space, range, perms)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        debug!("Process {} deleted", self.id);
    }
}
