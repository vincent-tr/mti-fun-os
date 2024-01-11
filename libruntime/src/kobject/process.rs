use core::ops::Range;

use libsyscalls::process;

use super::*;

/// Process
#[derive(Debug)]
pub struct Process {
    handle: Handle,
}

impl KObject for Process {
    unsafe fn handle(&self) -> &Handle {
        &self.handle
    }
}

impl Process {
    /// Get the current process
    pub fn current() -> &'static Self {
        lazy_static::lazy_static! {
          static ref CURRENT: Process = Process::init_current();
        }

        &CURRENT
    }

    fn init_current() -> Self {
        let handle = process::open_self().expect("Could not open current process");
        Self { handle }
    }

    /// Open the given process
    pub fn open(pid: u64) -> Result<Self, Error> {
        let handle = process::open(pid)?;

        Ok(Self { handle })
    }

    /// Reserve an area in the process VM, but no not back it with memory
    pub fn map_reserve(&self, addr: Option<usize>, size: usize) -> Result<Mapping, Error> {
        let addr = process::mmap(&self.handle, addr, size, Permissions::NONE, None, 0)?;

        Ok(unsafe { Mapping::unleak(self, addr..(addr + size), Permissions::NONE) })
    }

    /// Map a memory object into the process VM
    pub fn map_mem(
        &self,
        addr: Option<usize>,
        size: usize,
        perms: Permissions,
        mobj: &MemoryObject,
        offset: usize,
    ) -> Result<Mapping, Error> {
        let addr = process::mmap(
            &self.handle,
            addr,
            size,
            perms,
            Some(unsafe { mobj.handle() }),
            offset,
        )?;

        Ok(unsafe { Mapping::unleak(self, addr..(addr + size), perms) })
    }

    /// Unmap an area in the process VM
    pub fn unmap(&self, range: &Range<usize>) -> Result<(), Error> {
        process::munmap(&self.handle, range)
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

    /// Get the range of the mapping
    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// Get the start address of the mapping
    pub fn address(&self) -> usize {
        self.range.start
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
