// same api than kobject

use core::{ops::Range, ptr, slice};

use bitflags::bitflags;

bitflags! {
    /// Possible paging permissions
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct Permissions: u64 {
        /// No access
        const NONE = 0;

        /// Page can be read
        const READ = 1 << 0;

        /// Page can be written
        const WRITE = 1 << 1;

        /// Page can be executed
        const EXECUTE = 1 << 2;
    }
}

/// List of errors
#[derive(Debug)]
#[repr(usize)]
pub enum Error {
    InvalidArgument = 1,
    OutOfMemory,
    NotSupported,
    MemoryAccessDenied,
    ObjectNotFound,
    ObjectNameDuplicate,
    ObjectClosed,
    ObjectNotReady,
}
pub struct Process {
    _priv: (),
}

impl Process {
    /// Get the current process
    pub fn current() -> &'static Self {
        lazy_static::lazy_static! {
          static ref CURRENT: Process = Process { _priv: ()};
        }

        &CURRENT
    }

    /// Reserve an area in the process VM, but no not back it with memory
    pub fn map_reserve(&self, addr: Option<usize>, size: usize) -> Result<Mapping, Error> {
        let addr = addr.unwrap_or_default() as *mut _;

        let addr = unsafe {
            libc::mmap(
                addr,
                size,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if addr == ptr::null_mut() {
            return Err(Error::InvalidArgument);
        }

        let addr = addr as usize;

        Ok(unsafe { Mapping::unleak(self, addr..(addr + size), Permissions::NONE) })
    }

    /// Map a memory object into the process VM
    pub fn map_mem(
        &self,
        addr: Option<usize>,
        size: usize,
        perms: Permissions,
        //mobj: &MemoryObject,
        //offset: usize,
    ) -> Result<Mapping, Error> {
        let cperms = cperms(perms);
        let addr = addr.unwrap_or_default() as *mut _;

        let addr = unsafe {
            libc::mmap(
                addr,
                size,
                cperms,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if addr == ptr::null_mut() {
            return Err(Error::InvalidArgument);
        }

        let addr = addr as usize;

        Ok(unsafe { Mapping::unleak(self, addr..(addr + size), perms) })
    }

    /// Unmap an area in the process VM
    pub fn unmap(&self, range: &Range<usize>) -> Result<(), Error> {
        let addr = range.start as *mut _;

        let ret = unsafe { libc::munmap(addr, range.len()) };

        if ret == 0 {
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }
}

/// Mapping of memory
///
/// Note: creating an overlapping mapping will not update this one. Care must be taken to arrange it properly.
pub struct Mapping<'a> {
    process: &'a Process,
    range: Range<usize>,
    perms: Permissions,
}

fn cperms(perms: Permissions) -> libc::c_int {
    let mut value = libc::PROT_NONE;

    if perms.contains(Permissions::EXECUTE) {
        value |= libc::PROT_EXEC;
    }

    if perms.contains(Permissions::WRITE) {
        value |= libc::PROT_WRITE;
    }

    if perms.contains(Permissions::READ) {
        value |= libc::PROT_READ;
    }

    value
}

impl<'a> Mapping<'a> {
    /// Rebuild a mapping previously leaked
    ///
    /// # Safety
    ///
    /// The given arguments must be from a leaking mapping
    pub unsafe fn unleak(process: &'a Process, range: Range<usize>, perms: Permissions) -> Self {
        Self {
            process,
            range,
            perms,
        }
    }

    /// Is the mapping a reservation only?
    pub fn is_reservation(&self) -> bool {
        self.perms == Permissions::NONE
    }

    /// Get the permissions of the mapping
    pub fn permissions(&self) -> Permissions {
        self.perms
    }

    /// Update the permissions of the mapping
    ///
    /// Note: since the underlying memory object is not changed, Permissions cannot be switch with NONE
    ///
    /// # Safety
    ///
    /// Changing permissions on currently used mapping can result in page faults
    pub unsafe fn update_permissions(&mut self, perms: Permissions) -> Result<(), Error> {
        let cperms = cperms(perms);
        let addr = self.range.start as *mut _;

        let ret = unsafe { libc::mprotect(addr, self.range.len(), cperms) };

        if ret != 0 {
            return Err(Error::InvalidArgument);
        }

        self.perms = perms;
        Ok(())
    }

    /// Get the range of the mapping
    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// Get the start address of the mapping
    pub fn address(&self) -> usize {
        self.range.start
    }

    /// Get access to the mapping's data
    ///
    /// # Safety
    ///
    /// The slice remains valid as long as the mapping is not updated (eg: permissions)
    pub unsafe fn as_buffer(&self) -> Option<&'a [u8]> {
        if self.perms.contains(Permissions::READ) {
            Some(slice::from_raw_parts(
                self.address() as *const _,
                self.len(),
            ))
        } else {
            None
        }
    }

    /// Get access to the mapping's data
    ///
    /// # Safety
    ///
    /// The slice remains valid as long as the mapping is not updated (eg: permissions)
    pub unsafe fn as_buffer_mut(&self) -> Option<&'a mut [u8]> {
        if self.perms.contains(Permissions::WRITE) {
            Some(slice::from_raw_parts_mut(
                self.address() as *mut _,
                self.len(),
            ))
        } else {
            None
        }
    }

    /// Get the length in bytes of the mapping
    pub fn len(&self) -> usize {
        self.range.len()
    }

    /// Leak the mapping, consuming the object. The mapping is not freed.
    pub fn leak(mut self) {
        self.range = 0..0;
    }
}

impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        // Check not leaked
        if self.range.len() > 0 {
            self.process
                .unmap(&self.range)
                .expect("Could not free maping");
        }
    }
}
