use lazy_static::lazy_static;

use alloc::{string::String, sync::Arc, vec::Vec};
use syscalls::Error;

use crate::user::{
    error::{check_arg, duplicate_name},
    id_gen::IdGen,
    weak_map::WeakMap,
};

use super::{Port, PortReceiver, PortSender, port, port_access::access};

lazy_static! {
    pub static ref PORTS: Ports = Ports::new();
}

#[derive(Debug)]
pub struct Ports {
    id_gen: IdGen,
    ports: WeakMap<u64, PortSender>,
    names_map: WeakMap<String, PortSender>, // Only contains named ports
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
    /// Note: if specified, port name must be unique
    pub fn create(
        &self,
        name: Option<&str>,
    ) -> Result<(Arc<PortReceiver>, Arc<PortSender>), Error> {
        let name_str = name.map(String::from);

        if let Some(name_str) = &name_str {
            check_arg(name_str.len() > 0)?;

            // Forbid name duplicates here
            if self.names_map.has(&name_str) {
                return Err(duplicate_name());
            }
        }

        let id = self.id_gen.generate();
        let port = port::new(id, name);
        let (receiver, sender) = access(port);

        if let Some(name_str) = name_str {
            // TODO: thread safety
            self.names_map.insert(name_str, &sender);
        }

        self.ports.insert(id, &sender);

        Ok((receiver, sender))
    }

    /// Port drop
    fn remove(&self, port: &Port) {
        self.ports.remove(port.id());

        if let Some(name) = port.name() {
            self.names_map.remove(String::from(name));
        }
    }

    /// Find a port by its id
    pub fn find_by_id(&self, id: u64) -> Option<Arc<PortSender>> {
        self.ports.find(&id)
    }

    /// Find a port by its name
    pub fn find_by_name(&self, name: &str) -> Option<Arc<PortSender>> {
        self.names_map.find(&String::from(name))
    }

    /// List port ids
    pub fn list(&self) -> Vec<u64> {
        self.ports.keys()
    }
}

/// Reserved for port drop
pub fn remove_port(port: &Port) {
    PORTS.remove(port)
}
