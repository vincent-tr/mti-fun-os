mod mapping;
mod mappings;
mod memory_access;
mod process;
mod processes;

use alloc::sync::Arc;
use alloc::vec::Vec;

pub use self::memory_access::{MemoryAccess, TypedMemoryAccess};
pub use self::process::Process;
use self::processes::PROCESSES;

use super::Error;

pub fn create() -> Result<Arc<Process>, Error> {
    PROCESSES.create()
}

pub fn find(pid: u64) -> Option<Arc<Process>> {
    PROCESSES.find(pid)
}

pub fn list() -> Vec<u64> {
    PROCESSES.list()
}
