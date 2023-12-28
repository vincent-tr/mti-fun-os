mod message;
mod port;
mod ports;

use alloc::{sync::Arc, vec::Vec};
use syscalls::Error;

pub use self::message::Message;
pub use self::port::Port;
use self::ports::PORTS;

pub fn create(name: &str) -> Result<Arc<Port>, Error> {
    PORTS.create(name)
}

pub fn find_by_id(id: u64) -> Option<Arc<Port>> {
    PORTS.find_by_id(id)
}

pub fn find_by_name(name: &str) -> Option<Arc<Port>> {
    PORTS.find_by_name(name)
}

pub fn list() -> Vec<u64> {
    PORTS.list()
}
