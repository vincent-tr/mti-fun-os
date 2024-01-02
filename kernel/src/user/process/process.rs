use core::ops::Range;

use alloc::{string::String, sync::Arc};
use log::debug;
use spin::RwLock;

use crate::{
    memory::{create_adress_space, AddressSpace, AllocatorError, Permissions, VirtAddr},
    user::{error::check_any_permissions, handle::Handles, thread::Thread, weak_map::WeakMap},
};

use super::{
    mapping::Mapping,
    mappings::Mappings,
    memory_access::{self, TypedMemoryAccess, TypedSliceMemoryAccess},
    MemoryAccess,
};

use crate::user::{
    error::{check_arg, check_is_userspace, check_page_alignment, check_positive, out_of_memory},
    Error, MemoryObject,
};

/// Standalone function, so that Process::new() can remain private
///
/// Note: Only Process type is exported by process module, not this function
pub fn new(id: u64, name: &str) -> Result<Arc<Process>, Error> {
    Process::new(id, name)
}

/// Process
#[derive(Debug)]
pub struct Process {
    id: u64,
    name: String,
    address_space: RwLock<AddressSpace>,
    /// Note: ordered by address
    mappings: RwLock<Mappings>,
    threads: WeakMap<u64, Thread>,
    handles: Handles,
}

impl Process {
    fn new(id: u64, name: &str) -> Result<Arc<Self>, Error> {
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
            name: String::from(name),
            address_space: RwLock::new(address_space),
            mappings: RwLock::new(Mappings::new()),
            threads: WeakMap::new(),
            handles: Handles::new(),
        });

        debug!("Process {} created", process.id);

        Ok(process)
    }

    /// Get the process identifier
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the process name
    pub fn name<'a>(&'a self) -> &'a str {
        &self.name
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
    pub fn mmap(
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
    ///
    /// Notes:
    /// - It may contains multiple mappings,
    /// - addr or addr+size may be in the middle of a mapping
    /// - part of the specified area my not be mapped. In consequence, calling unmap() on an unmapped area is a successful noop.
    ///
    pub fn munmap(&self, addr: VirtAddr, size: usize) -> Result<(), Error> {
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

    /// Change the permissions for the given memory region
    ///
    /// Notes:
    /// - It can only contains one mapping
    /// - The mapping may be larger than the given region. It will be split.
    pub fn mprotect(&self, addr: VirtAddr, size: usize, perms: Permissions) -> Result<(), Error> {
        check_positive(size)?;
        check_page_alignment(size)?;
        check_is_userspace(addr)?;
        check_page_alignment(addr.as_u64() as usize)?;
        check_is_userspace(addr + size)?;
        check_any_permissions(perms)?;

        let mut mappings = self.mappings.write();

        let range = addr..addr + size;

        check_arg(mappings.is_contigous_mapping(&range))?;

        mappings.update_access_range(range.clone(), perms);

        debug!(
            "Process {}: mprotect at {:?} -> {:?}",
            self.id, range, perms
        );

        Ok(())
    }

    /// Create a new memory access to a part of the process VM
    ///
    /// permissions are at least expected permission in address space.
    ///
    /// eg: if READ is set, then the range must be mapped in the address space with at least READ permission
    ///
    /// Note that the kernel access itself it always READ/WRITE
    pub fn vm_access(
        &self,
        range: Range<VirtAddr>,
        perms: Permissions,
    ) -> Result<MemoryAccess, Error> {
        let address_space = self.address_space().read();
        memory_access::create(&address_space, range, perms)
    }

    /// Same than `vm_access`, but with typed data (easier access)
    pub fn vm_access_typed<T>(
        &self,
        addr: VirtAddr,
        perms: Permissions,
    ) -> Result<TypedMemoryAccess<T>, Error> {
        let address_space = self.address_space().read();
        memory_access::create_typed(&address_space, addr, perms)
    }

    /// Same than `vm_access`, but with typed slice data (easier access)
    pub fn vm_access_typed_slice<T>(
        &self,
        addr: VirtAddr,
        count: usize,
        perms: Permissions,
    ) -> Result<TypedSliceMemoryAccess<T>, Error> {
        let address_space = self.address_space().read();
        memory_access::create_typed_slice(&address_space, addr, count, perms)
    }

    /// Add a thread to the process
    pub fn add_thread(&self, thread: &Arc<Thread>) {
        self.threads.insert(thread.id(), thread);
    }

    /// Get the handle manager of the process
    pub fn handles(&self) -> &Handles {
        &self.handles
    }

    /// Get the number of threads in the process
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }

    /// Get the number of mappings in the address space of the process
    pub fn mapping_count(&self) -> usize {
        let mappings = self.mappings.read();

        mappings.len()
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        debug!("Process {} deleted", self.id);
    }
}
