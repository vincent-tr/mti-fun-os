use core::{mem, slice};

use bit_field::BitField;
use libruntime::net::types::IpAddress;
use log::{debug, warn};

use crate::{
    iface::Interface,
    packet::{Packet, PacketBuilder, PacketCursor},
};

use super::*;

/// Ip header struct, representing the header of an Ip packet.
#[derive(Debug)]
#[repr(packed)]
struct IpHeader {
    version_ihl: VersionIhl,
    dscp_ecn: u8,
    total_length: NetU16,
    identification: NetU16,
    flags_fragment_offset: NetU16,
    ttl: u8,
    protocol: u8,
    header_checksum: NetU16,
    source: IpAddress,
    destination: IpAddress,
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct VersionIhl(u8);

impl VersionIhl {
    pub fn version(&self) -> usize {
        self.0.get_bits(4..8) as usize
    }

    pub fn set_version(&mut self, value: usize) {
        assert!(value <= 0x0F, "Version value must fit in 4 bits");
        self.0.set_bits(4..8, value as u8);
    }

    pub fn ihl(&self) -> usize {
        self.0.get_bits(0..4) as usize
    }

    pub fn set_ihl(&mut self, value: usize) {
        assert!(value <= 0x0F, "IHL value must fit in 4 bits");
        self.0.set_bits(0..4, value as u8);
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct FlagsFragmentOffset(NetU16);

impl FlagsFragmentOffset {
    pub fn df(&self) -> bool {
        self.0.to_u16().get_bit(1)
    }

    pub fn set_df(&mut self, value: bool) {
        let mut field_value = self.0.to_u16();
        field_value.set_bit(1, value);
        self.0 = NetU16::from_u16(field_value);
    }

    pub fn mf(&self) -> bool {
        self.0.to_u16().get_bit(2)
    }

    pub fn set_mf(&mut self, value: bool) {
        let mut field_value = self.0.to_u16();
        field_value.set_bit(2, value);
        self.0 = NetU16::from_u16(field_value);
    }

    pub fn fragment_offset(&self) -> usize {
        self.0.to_u16().get_bits(4..16) as usize
    }

    pub fn set_fragment_offset(&mut self, value: usize) {
        assert!(value <= 0x1FFF, "Fragment offset value must fit in 13 bits");
        let mut field_value = self.0.to_u16();
        field_value.set_bits(3..16, value as u16);
        self.0 = NetU16::from_u16(field_value);
    }
}

/// Parsed Ip packet data, containing the source and destination IP addresses.
#[derive(Debug)]
pub struct IpMetadata {
    pub destination: IpAddress,
    pub source: IpAddress,
}

/// IP protocol implementation.
#[derive(Debug)]
pub struct Ip {}

impl Ip {
    /// ICMP protocol number.
    pub const ICMP: u8 = 1;

    /// TCP protocol number.
    pub const TCP: u8 = 6;

    /// UDP protocol number.
    pub const UDP: u8 = 17;

    /// Create a new IP protocol instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Process an incoming IP packet.
    pub fn receive(&self, iface: &Arc<Interface>, metadata: EthernetMetadata, packet: Packet) {
        if packet.len() < mem::size_of::<IpHeader>() {
            warn!(
                "[{}] Received packet too short to contain Ip header: length={} (dropped)",
                iface.name(),
                packet.len()
            );
            return;
        }

        let mut cursor = PacketCursor::new(&packet);
        let mut header = cursor.read::<IpHeader>().expect("Could not read ip header");

        if header.version_ihl.version() != 4 {
            warn!(
                "[{}] Received packet with unsupported IP version: version={} (dropped)",
                iface.name(),
                header.version_ihl.version()
            );
            return;
        }

        if header.version_ihl.ihl() < 5 {
            warn!(
                "[{}] Received packet with invalid IHL: ihl={} (dropped)",
                iface.name(),
                header.version_ihl.ihl()
            );
            return;
        }

        if packet.len() < header.total_length.to_u16() as usize {
            warn!(
                "[{}] Received packet with total length larger than actual length: total_length={} (dropped)",
                iface.name(),
                header.total_length.to_u16()
            );
            return;
        }

        let expected_checksum = self.compute_checksum(&mut header);
        if header.header_checksum.to_u16() != expected_checksum {
            warn!(
                "[{}] Received packet with invalid header checksum: header_checksum={:#04x}, expected={:#04x} (dropped)",
                iface.name(),
                header.header_checksum.to_u16(),
                expected_checksum
            );
            return;
        }

        let payload = cursor
            .read_data(header.total_length.to_u16() as usize - mem::size_of::<IpHeader>())
            .expect("Could not read ip payload");

        let metadata = IpMetadata {
            destination: header.destination,
            source: header.source,
        };

        match header.protocol {
            Self::ICMP => GlobalProtocols::instance()
                .icmp()
                .receive(metadata, payload),
            Self::TCP => GlobalProtocols::instance().tcp().receive(metadata, payload),
            Self::UDP => GlobalProtocols::instance().udp().receive(metadata, payload),
            protocol => warn!(
                "[{}] Received packet with unsupported protocol: protocol={}, from={} (dropped)",
                iface.name(),
                protocol,
                metadata.source
            ),
        }
    }

    /// Send an IP packet to the specified destination.
    pub async fn send(&self, destination: IpAddress, protocol: u8, mut packet: PacketBuilder) {
        todo!()
    }

    fn compute_checksum(&self, header: &mut IpHeader) -> u16 {
        // Backup the original checksum value before setting it to zero for calculation
        let checksum_backup = header.header_checksum;
        header.header_checksum = NetU16::from_u16(0);

        let buffer = unsafe {
            slice::from_raw_parts(
                (header as *const IpHeader) as *const u8,
                mem::size_of::<IpHeader>(),
            )
        };

        let mut sum: u32 = 0;
        let mut chunks = buffer.chunks_exact(2);

        for chunk in &mut chunks {
            let word = u16::from_be_bytes([chunk[0], chunk[1]]);
            sum += word as u32;
        }

        if let Some(&last) = chunks.remainder().first() {
            sum += (last as u32) << 8;
        }

        // Restore the original checksum value
        header.header_checksum = checksum_backup;

        while (sum >> 16) != 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }

        !(sum as u16)
    }
}
