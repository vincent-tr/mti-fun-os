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
