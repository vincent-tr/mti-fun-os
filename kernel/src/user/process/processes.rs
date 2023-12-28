use lazy_static::lazy_static;

use alloc::{sync::Arc, vec::Vec};

use crate::user::{id_gen::IdGen, process::process, weak_map::WeakMap, Error};

use super::Process;

lazy_static! {
    pub static ref PROCESSES: Processes = Processes::new();
}

#[derive(Debug)]
pub struct Processes {
    id_gen: IdGen,
    processes: WeakMap<u64, Process>,
}

impl Processes {
    fn new() -> Self {
        Self {
            id_gen: IdGen::new(),
            processes: WeakMap::new(),
        }
    }

    /// Create a new process
    pub fn create(&self, name: &str) -> Result<Arc<Process>, Error> {
        let id = self.id_gen.generate();
        let process = process::new(id, name)?;

        self.processes.insert(id, &process);

        Ok(process)
    }

    /// Find a process by its pid
    pub fn find(&self, pid: u64) -> Option<Arc<Process>> {
        self.processes.find(&pid)
    }

    /// List pids
    pub fn list(&self) -> Vec<u64> {
        self.processes.keys()
    }
}
