use libruntime::net::types::IpAddress;
use log::{debug, warn};

use crate::{
    iface::Interface,
    packet::{Packet, PacketBuilder, PacketCursor},
};

use super::*;

/// IP protocol implementation.
#[derive(Debug)]
pub struct Ip {}

impl Ip {
    /// Create a new IP protocol instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Process an incoming IP packet.
    pub fn receive(&self, iface: &Arc<Interface>, metadata: EthernetMetadata, packet: Packet) {
        todo!()
    }

    /// Send an IP packet to the specified destination.
    pub async fn send(&self, destination: IpAddress, mut packet: PacketBuilder) {
        todo!()
    }
}
