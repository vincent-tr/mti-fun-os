use core::{
    fmt, mem, slice,
    sync::atomic::{AtomicU16, Ordering},
};

use alloc::vec::Vec;
use bit_field::BitField;
use libruntime::{net::types::IpAddress, sync::RwLock};
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
    flags_fragment_offset: FlagsFragmentOffset,
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
    pub fn new(version: usize, ihl: usize) -> Self {
        let mut value = VersionIhl(0);
        value.set_version(version);
        value.set_ihl(ihl);
        value
    }

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
    pub fn new(df: bool, mf: bool, fragment_offset: usize) -> Self {
        let mut value = FlagsFragmentOffset(NetU16::ZERO);
        value.set_df(df);
        value.set_mf(mf);
        value.set_fragment_offset(fragment_offset);
        value
    }

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

#[derive(Debug)]
pub struct IpPrefix {
    network: IpAddress,
    prefix_len: usize, // 0..=32
}

impl IpPrefix {
    /// Create a new prefix
    pub fn new(network: IpAddress, prefix_len: usize) -> Self {
        assert!(prefix_len <= 32);
        Self {
            network,
            prefix_len,
        }
    }

    /// Test if the given IP address is part of the network
    pub fn matches(&self, ip: IpAddress) -> bool {
        let mask = if self.prefix_len == 0 {
            0
        } else {
            u32::MAX << (32 - self.prefix_len)
        };

        let net = self.network.as_u32();
        let dest = ip.as_u32();

        (net & mask) == (dest & mask)
    }
}

impl fmt::Display for IpPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix_len)
    }
}

#[derive(Debug)]
pub enum NextHop {
    Direct,             // on-link → ARP(dst)
    Gateway(IpAddress), // via gw → ARP(gw)
}

#[derive(Debug)]
pub struct Route {
    prefix: IpPrefix,
    next_hop: NextHop,
    iface: Arc<Interface>,
    metric: usize, // lower = preferred
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Destination
        if self.prefix.prefix_len == 0 {
            write!(f, "default")?;
        } else {
            write!(f, "{}", self.prefix)?;
        }

        // Next hop
        match self.next_hop {
            NextHop::Direct => {
                write!(f, " dev {}", self.iface.name())?;
            }
            NextHop::Gateway(gw) => {
                write!(f, " via {} dev {}", gw, self.iface.name())?;
            }
        }

        // Metric
        if self.metric != 0 {
            write!(f, " metric {}", self.metric)?;
        }

        Ok(())
    }
}

/// IP protocol implementation.
#[derive(Debug)]

pub struct Ip {
    routes: RwLock<Vec<Arc<Route>>>,
    next_id: AtomicU16,
}

impl Ip {
    /// ICMP protocol number.
    pub const ICMP: u8 = 1;

    /// TCP protocol number.
    pub const TCP: u8 = 6;

    /// UDP protocol number.
    pub const UDP: u8 = 17;

    /// Version of IP protocol
    const IP_VERSION: usize = 4;

    /// TTL used in tx packets
    const TTL: u8 = 64;

    /// Create a new IP protocol instance.
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(Vec::new()),
            next_id: AtomicU16::new(0),
        }
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

        if header.version_ihl.version() != Self::IP_VERSION {
            warn!(
                "[{}] Received packet with unsupported IP version: version={} (dropped)",
                iface.name(),
                header.version_ihl.version()
            );
            return;
        }

        if header.version_ihl.ihl() < mem::size_of::<IpHeader>() / mem::size_of::<u32>() {
            warn!(
                "[{}] Received packet with invalid IHL: ihl={} (dropped)",
                iface.name(),
                header.version_ihl.ihl()
            );
            return;
        }

        if !Self::check_dest(&iface, header.destination) {
            debug!(
                "[{}] Received packet not destined for this interface: destination={}, source={} (dropped)",
                iface.name(),
                header.destination,
                header.source,
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

        if header.flags_fragment_offset.mf() || header.flags_fragment_offset.fragment_offset() > 0 {
            warn!(
                "[{}] Received packet with fragmentation (dropped)",
                iface.name(),
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

    fn check_dest(iface: &Interface, dest: IpAddress) -> bool {
        // Can receive broadcast even without ip config (this is how DHCP works)
        if dest.is_broadcast() {
            return true;
        }

        let Some(ip_config) = iface.ip_config() else {
            return false;
        };

        return ip_config.ip_address() == dest;
    }

    fn compute_checksum(&self, header: &mut IpHeader) -> u16 {
        // Backup the original checksum value before setting it to zero for calculation
        let checksum_backup = header.header_checksum;
        header.header_checksum = NetU16::ZERO;

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

    /// Send an IP packet to the specified destination.
    pub async fn send(&self, destination: IpAddress, protocol: u8, mut packet: PacketBuilder) {
        if packet.len() > Ethernet::MTU {
            error!(
                "Packet to big ({}) to fit Ethernet MTU ({}) (dropped)",
                packet.len(),
                Ethernet::MTU
            );
            return;
        }

        let route = {
            let routes = self.routes.read();

            let route = routes
                .iter()
                .filter(|route| route.prefix.matches(destination))
                .max_by_key(|route| (route.prefix.prefix_len, usize::MAX - route.metric))
                .cloned();

            let Some(route) = route else {
                error!("No route to send packet to {} (dropped)", destination,);
                return;
            };

            route
        };

        let next_hop = match route.next_hop {
            NextHop::Direct => destination,
            NextHop::Gateway(gateway) => gateway,
        };

        let source = route
            .iface
            .ip_config()
            .expect("Got interface without IP config")
            .ip_address();

        let header = IpHeader {
            version_ihl: VersionIhl::new(
                Self::IP_VERSION,
                mem::size_of::<IpHeader>() / mem::size_of::<u32>(),
            ),
            dscp_ecn: 0,
            total_length: NetU16::from_u16((mem::size_of::<IpHeader>() + packet.len()) as u16),
            identification: NetU16::from_u16(self.next_id.fetch_add(1, Ordering::Relaxed)),
            flags_fragment_offset: FlagsFragmentOffset::new(true, false, 0),
            ttl: Self::TTL,
            protocol,
            header_checksum: NetU16::ZERO,
            source,
            destination,
        };

        packet.prepend(header);

        route.iface.send_ip_packet(next_hop, packet).await;
    }
}
