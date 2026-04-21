use log::debug;

use crate::packet::Packet;

use super::*;

/// ICMP protocol implementation.
#[derive(Debug)]
pub struct Icmp {}

impl Icmp {
    /// Create a new ICMP protocol instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Process an incoming ICMP packet.
    pub fn receive(&self, metadata: IpMetadata, payload: Packet) {
        debug!(
            "Received ICMP packet from {} to {}",
            metadata.source, metadata.destination
        );
    }
}
