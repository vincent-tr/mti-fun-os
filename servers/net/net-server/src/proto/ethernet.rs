use core::mem;

use crc::{CRC_32_ISO_HDLC, Crc};
use libruntime::{debug, net::types::MacAddress};
use log::{debug, warn};

use crate::{
    iface::Interface,
    packet::{Packet, PacketCursor},
};

use super::{NetU16, NetU32};

/// Ethernet header struct, representing the header of an Ethernet frame.
#[derive(Debug)]
#[repr(packed)]
struct EthernetHeader {
    destination: MacAddress,
    source: MacAddress,
    ethertype: NetU16,
}

/// Ethernet footer struct, representing the footer of an Ethernet frame.
#[derive(Debug)]
#[repr(packed)]
struct EthernetFooter {
    checksum: NetU32,
}

const ETHERTYPE_IPV4: u16 = 0x0800;
const ETHERTYPE_ARP: u16 = 0x0806;

/// Process an incoming Ethernet frame, given the raw bytes of the frame.
pub async fn rx_packet(iface: &Interface, packet: Packet) {
    if packet.len() < mem::size_of::<EthernetHeader>() + mem::size_of::<EthernetFooter>() {
        warn!(
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

    // Note: QEmu does not provide FCS value in the Ethernet footer, so we cannot actually verify the checksum here.
    let _ = checksum;
    let _ = footer.checksum.to_u32();
    // if checksum != footer.checksum.to_u32() {
    //     warn!(
    //         "Received packet with invalid checksum: expected={:#010x}, actual={:#010x} (dropped)",
    //         footer.checksum.to_u32(),
    //         checksum
    //     );
    //     return;
    // }

    match header.ethertype.to_u16() {
        ETHERTYPE_IPV4 => {
            // TODO
            debug!(
                "Received IPv4 packet from {} to {}",
                header.source, header.destination
            );
        }
        ETHERTYPE_ARP => {
            // TODO
            debug!(
                "Received ARP packet from {} to {}",
                header.source, header.destination
            );
        }
        ethertype => {
            warn!(
                "Received packet with unknown ethertype {:#06x} from {} to {} (dropped)",
                ethertype, header.source, header.destination
            );
        }
    }
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
