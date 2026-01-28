use libsyscalls::memory_object;
use spin::Mutex;

use super::*;

/// Memory object
#[derive(Debug)]
pub struct MemoryObject {
    handle: Handle,
    cached_size: Mutex<Option<usize>>,
}

impl KObject for MemoryObject {
    unsafe fn handle(&self) -> &Handle {
        &self.handle
    }

    fn into_handle(self) -> Handle {
        self.handle
    }
}

impl MemoryObject {
    /// Safety: caller must ensure the handle is a valid memory object handle
    pub unsafe fn from_handle_unchecked(handle: Handle) -> Self {
        Self {
            handle,
            cached_size: Mutex::new(None),
        }
    }

    /// Create a new memory object of the specified size
    pub fn create(size: usize) -> Result<Self, Error> {
        let handle = memory_object::create(size)?;
        Ok(Self {
            handle,
            cached_size: Mutex::new(Some(size)),
        })
    }

    pub fn size(&self) -> Result<usize, Error> {
        let mut value = self.cached_size.lock();
        if let Some(size) = *value {
            return Ok(size);
        }

        let size = memory_object::size(&self.handle)?;
        *value = Some(size);
        Ok(size)
    }
}
