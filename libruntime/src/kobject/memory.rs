use libsyscalls::memory;

use super::*;

/// Memory
pub struct Memory {
    _priv: (),
}

impl Memory {
    /// Get stats on memory usage
    pub fn stats() -> MemoryStats {
        memory::stats().expect("Could not get memory stats")
    }
}
