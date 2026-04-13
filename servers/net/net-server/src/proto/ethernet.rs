use core::mem;

use libruntime::{debug, net::types::MacAddress};
use log::{debug, warn};

use crate::{
    iface::Interface,
    packet::{Packet, PacketCursor},
    proto::arp,
};

use super::NetU16;

/// Ethernet header struct, representing the header of an Ethernet frame.
#[derive(Debug)]
#[repr(packed)]
struct EthernetHeader {
    destination: MacAddress,
    source: MacAddress,
    ethertype: NetU16,
}

pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16 = 0x0806;

/// Process an incoming Ethernet frame, given the raw bytes of the frame.
pub async fn rx_packet(iface: &Interface, packet: Packet) {
    if packet.len() < mem::size_of::<EthernetHeader>() {
        warn!(
            "Received packet too short to contain Ethernet header: length={} (dropped)",
            packet.len()
        );
        return;
    }

    let mut cursor = PacketCursor::new(&packet);
    let header = cursor
        .read::<EthernetHeader>()
        .expect("Could not read ethernet header");

    let payload = cursor
        .read_data(packet.len() - mem::size_of::<EthernetHeader>())
        .expect("Could not read ethernet payload");

    assert!(
        cursor.is_end(),
        "Cursor should be at the end of the packet after reading header and payload"
    );

    match header.ethertype.to_u16() {
        ETHERTYPE_IPV4 => {
            // TODO
            debug!(
                "Received IPv4 packet from {} to {}",
                header.source, header.destination
            );
        }
        ETHERTYPE_ARP => arp::rx_packet(iface, payload).await,
        ethertype => warn!(
            "Received packet with unknown ethertype {:#06x} from {} to {} (dropped)",
            ethertype, header.source, header.destination
        ),
    }
}
