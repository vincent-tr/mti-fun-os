use log::debug;

use crate::packet::Packet;

use super::*;

/// TCP protocol implementation.
#[derive(Debug)]
pub struct Tcp {}

impl Tcp {
    /// Create a new TCP protocol instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Process an incoming TCP packet.
    pub fn receive(&self, metadata: IpMetadata, payload: Packet) {
        debug!(
            "Received TCP packet from {} to {}",
            metadata.source, metadata.destination
        );
    }
}
