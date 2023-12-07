mod mapping;
mod mappings;
mod process;
mod processes;

use alloc::sync::Arc;

use self::processes::PROCESSES;
pub use self::process::Process;

use super::Error;

pub fn create() -> Result<Arc<Process>, Error> {
    PROCESSES.create()
}

pub fn find(pid: u32) -> Option<Arc<Process>> {
    PROCESSES.find(pid)
}
