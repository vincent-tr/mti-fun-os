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
    pub fn map_reserve(&self, addr: Option<usize>, size: usize) -> Result<usize, Error> {
        process::mmap(&self.handle, addr, size, Permissions::NONE, None, 0)
    }

    /// Map a memory object into the process VM
    pub fn map_mem(
        &self,
        addr: Option<usize>,
        size: usize,
        perms: Permissions,
        mobj: &MemoryObject,
        offset: usize,
    ) -> Result<usize, Error> {
        process::mmap(
            &self.handle,
            addr,
            size,
            perms,
            Some(unsafe { mobj.handle() }),
            offset,
        )
    }

    /// Unmap an area in the process VM
    pub fn unmap(&self, range: &Range<usize>) -> Result<(), Error> {
        process::munmap(&self.handle, range)
    }
}
