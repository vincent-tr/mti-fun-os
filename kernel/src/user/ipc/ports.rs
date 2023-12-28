use lazy_static::lazy_static;

use alloc::{string::String, sync::Arc, vec::Vec};
use syscalls::Error;

use crate::user::{error::duplicate_name, id_gen::IdGen, weak_map::WeakMap};

use super::{port, Port};

lazy_static! {
    pub static ref PORTS: Ports = Ports::new();
}

#[derive(Debug)]
pub struct Ports {
    id_gen: IdGen,
    ports: WeakMap<u64, Port>,
    names_map: WeakMap<String, Port>, // Only contains named ports
}

impl Ports {
    fn new() -> Self {
        Self {
            id_gen: IdGen::new(),
            ports: WeakMap::new(),
            names_map: WeakMap::new(),
        }
    }

    /// Create a new port
    ///
    /// Note: port name must be unique
    pub fn create(&self, name: &str) -> Result<Arc<Port>, Error> {
        let id = self.id_gen.generate();
        let port = port::new(id, name);

        if name.len() > 0 {
            // Forbid name duplicates here
            let name = String::from(name);
            if self.names_map.has(&name) {
                return Err(duplicate_name());
            }

            self.names_map.insert(name, &port);
        }

        self.ports.insert(id, &port);

        Ok(port)
    }

    /// Find a port by its id
    pub fn find_by_id(&self, id: u64) -> Option<Arc<Port>> {
        self.ports.find(&id)
    }

    /// Find a port by its name
    pub fn find_by_name(&self, name: &str) -> Option<Arc<Port>> {
        self.names_map.find(&String::from(name))
    }

    /// List pids
    pub fn list(&self) -> Vec<u64> {
        self.ports.keys()
    }
}
