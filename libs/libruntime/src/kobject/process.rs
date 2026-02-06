use core::{ops::Range, slice};

use alloc::{boxed::Box, string::String, vec::Vec};
use libsyscalls::{process, HandleType};
use spin::Mutex;

use super::*;

/// Process
#[derive(Debug)]
pub struct Process {
    cached_pid: Mutex<Option<u64>>,
    handle: Handle,
}

impl KObject for Process {
    unsafe fn handle(&self) -> &Handle {
        &self.handle
    }

    fn into_handle(self) -> Handle {
        self.handle
    }

    unsafe fn from_handle_unchecked(handle: Handle) -> Self {
        Self {
            cached_pid: Mutex::new(None),
            handle,
        }
    }

    fn from_handle(handle: Handle) -> Result<Self, Error> {
        if !handle.valid() {
            return Err(Error::InvalidArgument);
        }
        if handle.r#type() != HandleType::Process {
            return Err(Error::InvalidArgument);
        }

        Ok(unsafe { Self::from_handle_unchecked(handle) })
    }
}

impl Clone for Process {
    fn clone(&self) -> Self {
        let handle = self.handle.clone();

        Self {
            cached_pid: Mutex::new(*self.cached_pid.lock()),
            handle,
        }
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
        Self {
            cached_pid: Mutex::new(None),
            handle,
        }
    }

    /// Create a new process
    pub fn create(name: &str) -> Result<Self, Error> {
        let handle = process::create(name)?;

        Ok(Self {
            cached_pid: Mutex::new(None),
            handle,
        })
    }

    /// Open the given process
    pub fn open(pid: u64) -> Result<Self, Error> {
        let handle = process::open(pid)?;

        Ok(Self {
            cached_pid: Mutex::new(Some(pid)),
            handle,
        })
    }

    /// Get the process id
    pub fn pid(&self) -> u64 {
        if let Some(value) = *self.cached_pid.lock() {
            return value;
        }

        // Will also fill cache
        let info = self.info();
        info.pid
    }

    /// Get process info
    pub fn info(&self) -> ProcessInfo {
        let info = process::info(&self.handle).expect("Could not get process info");

        {
            let mut cached_pid = self.cached_pid.lock();

            if cached_pid.is_none() {
                *cached_pid = Some(info.pid);
            }
        }

        info
    }

    /// List the process ids in the system
    pub fn list() -> Result<Box<[u64]>, Error> {
        let mut size = 1024;

        // Event not atomic, let's consider that with doubling the required size between call,
        // at some point we will be able to fetch list entirely
        loop {
            let mut buffer = Vec::with_capacity(size);
            buffer.resize(size, 0);

            let (_, new_size) = process::list(&mut buffer)?;

            if new_size > size {
                // Retry with 2x requested size
                size = new_size * 2;
                continue;
            }

            buffer.resize(new_size, 0);

            return Ok(buffer.into_boxed_slice());
        }
    }

    /// Set the name of the process
    pub fn set_name(&self, name: &str) -> Result<(), Error> {
        process::set_name(&self.handle, name)?;

        Ok(())
    }

    /// Get the name of the process
    pub fn name(&self) -> Result<String, Error> {
        let mut size = ProcessInfo::NAME_LEN;

        // Even if not atomic, let's consider we won't have many tries before we get a correct size
        loop {
            let mut buffer = Vec::with_capacity(size);
            buffer.resize(size, 0);

            let (_, new_size) = process::get_name(&self.handle, &mut buffer)?;

            if new_size > size {
                // Retry
                size = new_size;
                continue;
            }

            buffer.resize(new_size, 0);

            return Ok(unsafe { String::from_utf8_unchecked(buffer) });
        }
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
        process::munmap(&self.handle, range)?;

        Ok(())
    }

    pub fn map_info(&self, addr: usize) -> Result<AddressInfo, Error> {
        let info = process::minfo(&self.handle, addr)?;

        let mobj = info
            .mobj
            .map(|handle| unsafe { MemoryObject::from_handle_unchecked(handle) });

        Ok(AddressInfo {
            perms: info.perms,
            mobj,
            offset: info.offset,
        })
    }

    pub fn kill(&self) -> Result<(), Error> {
        process::kill(&self.handle)?;

        Ok(())
    }
}

/// Address info in a process
#[derive(Debug)]
pub struct AddressInfo {
    pub perms: Permissions,
    pub mobj: Option<MemoryObject>,
    pub offset: usize, // Offset inside the memory object, if any
}

/// Mapping of memory
///
/// Note: creating an overlapping mapping will not update this one. Care must be taken to arrange it properly.
#[derive(Debug)]
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

    /// Update the permissions of the mapping
    ///
    /// Note: since the underlying memory object is not changed, Permissions cannot be switch with NONE
    ///
    /// # Safety
    ///
    /// Changing permissions on currently used mapping can result in page faults
    pub unsafe fn update_permissions(&mut self, perms: Permissions) -> Result<(), Error> {
        process::mprotect(&self.process.handle, &self.range, perms)?;
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
        if self.process.pid() != Process::current().pid() {
            return None;
        }
        if !self.perms.contains(Permissions::READ) {
            return None;
        }

        Some(slice::from_raw_parts(
            self.address() as *const _,
            self.len(),
        ))
    }

    /// Get access to the mapping's data
    ///
    /// # Safety
    ///
    /// The slice remains valid as long as the mapping is not updated (eg: permissions)
    pub unsafe fn as_buffer_mut(&self) -> Option<&'a mut [u8]> {
        if self.process.pid() != Process::current().pid() {
            return None;
        }
        if !self.perms.contains(Permissions::WRITE) {
            return None;
        }

        Some(slice::from_raw_parts_mut(
            self.address() as *mut _,
            self.len(),
        ))
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
