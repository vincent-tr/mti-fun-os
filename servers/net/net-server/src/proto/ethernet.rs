use core::mem;

use crc::{CRC_32_ISO_HDLC, Crc};
use libruntime::{debug, net::types::MacAddress};
use log::{debug, error};

use crate::packet::{Packet, PacketCursor};

use super::{NetU16, NetU32};

/// Ethernet header struct, representing the header of an Ethernet frame.
#[derive(Debug)]
#[repr(packed)]
struct EthernetHeader {
    destination: MacAddress,
    source: MacAddress,
    ethertype: NetU16,
}

struct EthernetFooter {
    checksum: NetU32,
}

/// Process an incoming Ethernet frame, given the raw bytes of the frame.
pub async fn rx_packet(packet: Packet) {
    if packet.len() < mem::size_of::<EthernetHeader>() + mem::size_of::<EthernetFooter>() {
        error!(
            "Received packet too short to contain Ethernet header and footer: length={} (dropped)",
            packet.len()
        );
        return;
    }

    let mut cursor = PacketCursor::new(&packet);
    let header = cursor
        .read::<EthernetHeader>()
        .expect("Could not read ethernet header");

    let payload = cursor
        .read_data(
            packet.len() - mem::size_of::<EthernetHeader>() - mem::size_of::<EthernetFooter>(),
        )
        .expect("Could not read ethernet payload");

    let footer = cursor
        .read::<EthernetFooter>()
        .expect("Could not read ethernet footer");

    assert!(
        cursor.is_end(),
        "Cursor should be at the end of the packet after reading header, payload, and footer"
    );

    let checksum_payload = packet.slice(0..packet.len() - mem::size_of::<EthernetFooter>());
    let checksum = compute_checksum(&checksum_payload);

    if checksum != footer.checksum.to_u32() {
        error!(
            "Received packet with invalid checksum: expected={:#010x}, actual={:#010x} (dropped)",
            footer.checksum.to_u32(),
            checksum
        );
        return;
    }

    // TODO: process payload based on ethertype in header
}

/// Computes the CRC32 checksum of the given packet data, using the same algorithm as used by Ethernet frames.
fn compute_checksum(data: &Packet) -> u32 {
    const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

    let mut digest = CRC32.digest();
    let mut cursor = PacketCursor::new(data);

    while !cursor.is_end() {
        let chunk = cursor.read_chunk();
        digest.update(chunk);
    }

    digest.finalize()
}
