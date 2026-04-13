use core::mem;

use libruntime::net::types::{IpAddress, MacAddress};
use log::{debug, warn};

use crate::{
    iface::Interface,
    packet::{Packet, PacketCursor},
};

use super::NetU16;

#[derive(Debug)]
#[repr(packed)]
struct ArpPacket {
    pub htype: NetU16,
    pub ptype: NetU16,
    pub hlen: u8,
    pub plen: u8,
    pub oper: NetU16,

    pub sha: MacAddress,
    pub spa: IpAddress,
    pub tha: MacAddress,
    pub tpa: IpAddress,
}

/// Process an incoming ARP packet, given the raw bytes of the packet.
pub async fn rx_packet(iface: &Interface, packet: Packet) {
    if packet.len() < mem::size_of::<ArpPacket>() {
        warn!(
            "Received packet too short to contain ARP header: length={} (dropped)",
            packet.len()
        );
        return;
    }

    let mut cursor = PacketCursor::new(&packet);
    let arp_packet = cursor
        .read::<ArpPacket>()
        .expect("Could not read ARP packet");

    debug!(
        "Received ARP packet: htype={}, ptype={}, hlen={}, plen={}, oper={}, sha={}, spa={}, tha={}, tpa={}",
        arp_packet.htype.to_u16(),
        arp_packet.ptype.to_u16(),
        arp_packet.hlen,
        arp_packet.plen,
        arp_packet.oper.to_u16(),
        arp_packet.sha,
        arp_packet.spa,
        arp_packet.tha,
        arp_packet.tpa
    );
}
