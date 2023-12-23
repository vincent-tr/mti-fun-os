mod mapping;
mod mappings;
mod process;
mod processes;
mod memory_access;

use alloc::sync::Arc;

use self::processes::PROCESSES;
pub use self::process::Process;
pub use self::memory_access::MemoryAccess;

use super::Error;

pub fn create() -> Result<Arc<Process>, Error> {
    PROCESSES.create()
}

pub fn find(pid: u64) -> Option<Arc<Process>> {
    PROCESSES.find(pid)
}
