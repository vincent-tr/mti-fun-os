use core::mem;

use alloc::sync::Arc;
use libruntime::net::types::MacAddress;
use log::{debug, warn};

use crate::{
    iface::Interface,
    packet::{Packet, PacketCursor},
};

use super::*;

/// Ethernet header struct, representing the header of an Ethernet frame.
#[derive(Debug)]
#[repr(packed)]
struct EthernetHeader {
    destination: MacAddress,
    source: MacAddress,
    ethertype: NetU16,
}

/// Parsed Ethernet frame data, containing the source and destination MAC addresses.
#[derive(Debug)]
pub struct EthernetMetadata {
    pub destination: MacAddress,
    pub source: MacAddress,
}

/// Ethernet protocol implementation.
#[derive(Debug)]
pub struct Ethernet {
    iface: Arc<Interface>,
}

impl Ethernet {
    /// IPV4 ethernet type value.
    pub const IPV4: u16 = 0x0800;

    /// ARP ethernet type value.
    pub const ARP: u16 = 0x0806;

    /// Create a new Ethernet protocol instance.
    pub fn new(iface: Arc<Interface>) -> Self {
        Self { iface }
    }

    fn iface(&self) -> &Interface {
        &self.iface
    }

    fn name(&self) -> &str {
        self.iface().name()
    }

    /// Process an incoming Ethernet frame.
    pub async fn receive(&self, packet: Packet) {
        if packet.len() < mem::size_of::<EthernetHeader>() {
            warn!(
                "[{}] Received packet too short to contain Ethernet header: length={} (dropped)",
                self.name(),
                packet.len()
            );
            return;
        }

        let mut cursor = PacketCursor::new(&packet);
        let header = cursor
            .read::<EthernetHeader>()
            .expect("Could not read ethernet header");

        if !header.destination.is_broadcast() && header.destination != self.iface().mac_address() {
            debug!(
                "[{}] Received packet not destined for this interface: destination={}, source={} (dropped)",
                self.name(),
                header.destination,
                header.source
            );
            return;
        }

        let payload = cursor
            .read_data(packet.len() - mem::size_of::<EthernetHeader>())
            .expect("Could not read ethernet payload");

        assert!(
            cursor.is_end(),
            "Cursor should be at the end of the packet after reading header and payload"
        );

        let metadata = EthernetMetadata {
            destination: header.destination,
            source: header.source,
        };

        match header.ethertype.to_u16() {
            Self::IPV4 => {
                // TODO
                debug!(
                    "[{}] Received IPv4 packet from {} to {}",
                    self.name(),
                    metadata.source,
                    metadata.destination
                );
            }
            Self::ARP => {
                self.iface()
                    .protocols()
                    .arp()
                    .receive(metadata, payload)
                    .await
            }
            ethertype => warn!(
                "[{}] Received packet with unknown ethertype {:#06x} from {} to {} (dropped)",
                self.name(),
                ethertype,
                metadata.source,
                metadata.destination
            ),
        }
    }
}
