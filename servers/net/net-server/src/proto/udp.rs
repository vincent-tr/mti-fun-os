use log::debug;

use crate::packet::Packet;

use super::*;

/// UDP protocol implementation.
#[derive(Debug)]
pub struct Udp {}

impl Udp {
    /// Create a new UDP protocol instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Process an incoming UDP packet.
    pub fn receive(&self, metadata: IpMetadata, _payload: Packet) {
        debug!(
            "Received UDP packet from {} to {}",
            metadata.source, metadata.destination
        );
    }
}
