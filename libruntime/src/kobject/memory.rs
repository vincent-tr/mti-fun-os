use libsyscalls::memory_object;

use super::*;

/// Memory object
#[derive(Debug)]
pub struct MemoryObject {
    handle: Handle,
}

impl KObject for MemoryObject {
    unsafe fn handle(&self) -> &Handle {
        &self.handle
    }
}

impl MemoryObject {
    /// Create a new memory object of the specified size
    pub fn create(size: usize) -> Result<Self, Error> {
        let handle = memory_object::create(size)?;
        Ok(Self { handle })
    }
}
