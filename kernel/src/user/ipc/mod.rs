mod port;
mod port_access;
mod ports;

use alloc::{sync::Arc, vec::Vec};
use syscalls::Error;

pub use self::port::Port;
pub use self::port_access::{PortReceiver, PortSender};
use self::ports::PORTS;

pub fn create(name: Option<&str>) -> Result<(Arc<PortReceiver>, Arc<PortSender>), Error> {
    PORTS.create(name)
}

pub fn find_by_id(id: u64) -> Option<Arc<PortSender>> {
    PORTS.find_by_id(id)
}

pub fn find_by_name(name: &str) -> Option<Arc<PortSender>> {
    PORTS.find_by_name(name)
}

pub fn list() -> Vec<u64> {
    PORTS.list()
}
