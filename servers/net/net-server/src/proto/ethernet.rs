use core::mem;

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
pub struct Ethernet;

impl Ethernet {
    /// IPV4 ethernet type value.
    pub const IPV4: u16 = 0x0800;

    /// ARP ethernet type value.
    pub const ARP: u16 = 0x0806;

    /// Create a new Ethernet protocol instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Process an incoming Ethernet frame.
    pub async fn receive(&self, iface: &Interface, packet: Packet) {
        if packet.len() < mem::size_of::<EthernetHeader>() {
            warn!(
                "[{}] Received packet too short to contain Ethernet header: length={} (dropped)",
                iface.name(),
                packet.len()
            );
            return;
        }

        let mut cursor = PacketCursor::new(&packet);
        let header = cursor
            .read::<EthernetHeader>()
            .expect("Could not read ethernet header");

        if !header.destination.is_broadcast() && header.destination != iface.mac_address() {
            debug!(
                "[{}] Received packet not destined for this interface: destination={}, source={} (dropped)",
                iface.name(),
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
                    iface.name(),
                    metadata.source,
                    metadata.destination
                );
            }
            Self::ARP => {
                iface
                    .protocols()
                    .arp()
                    .receive(iface, metadata, payload)
                    .await
            }
            ethertype => warn!(
                "[{}] Received packet with unknown ethertype {:#06x} from {} to {} (dropped)",
                iface.name(),
                ethertype,
                metadata.source,
                metadata.destination
            ),
        }
    }
}
