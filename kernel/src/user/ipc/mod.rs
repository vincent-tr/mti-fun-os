mod message;
mod port;
mod port_access;
mod ports;

use alloc::{sync::Arc, vec::Vec};
use syscalls::Error;

pub use self::message::Message;
pub use self::port::Port;
use self::port_access::{PortReceiver, PortSender};
use self::ports::PORTS;

pub fn create(name: &str) -> Result<(Arc<PortReceiver>, Arc<PortSender>), Error> {
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
