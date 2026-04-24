use core::{
    mem,
    sync::atomic::{AtomicU16, Ordering},
};

use alloc::{string::String, vec::Vec};
use bit_field::BitField;
use libruntime::{
    net::{
        iface,
        types::{IpAddress, IpPrefix},
    },
    sync::RwLock,
};
use log::{debug, warn};

use crate::{
    iface::Interface,
    packet::{Packet, PacketBuilder, PacketCursor},
};

use super::*;

/// Ip header struct, representing the header of an Ip packet.
#[derive(Debug)]
#[repr(packed)]
#[allow(dead_code)]
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

#[allow(dead_code)]
impl FlagsFragmentOffset {
    pub fn new(df: bool, mf: bool, fragment_offset: usize) -> Self {
        let mut value = FlagsFragmentOffset(NetU16::ZERO);
        value.set_df(df);
        value.set_mf(mf);
        value.set_fragment_offset(fragment_offset);
        value
    }

    pub fn df(&self) -> bool {
        self.0.to_u16().get_bit(14)
    }

    pub fn set_df(&mut self, value: bool) {
        let mut field_value = self.0.to_u16();
        field_value.set_bit(14, value);
        self.0 = NetU16::from_u16(field_value);
    }

    pub fn mf(&self) -> bool {
        self.0.to_u16().get_bit(13)
    }

    pub fn set_mf(&mut self, value: bool) {
        let mut field_value = self.0.to_u16();
        field_value.set_bit(13, value);
        self.0 = NetU16::from_u16(field_value);
    }

    pub fn fragment_offset(&self) -> usize {
        self.0.to_u16().get_bits(0..13) as usize
    }

    pub fn set_fragment_offset(&mut self, value: usize) {
        assert!(value <= 0x1FFF, "Fragment offset value must fit in 13 bits");
        let mut field_value = self.0.to_u16();
        field_value.set_bits(0..13, value as u16);
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
enum NextHop {
    Direct,             // on-link → ARP(dst)
    Gateway(IpAddress), // via gw → ARP(gw)
}

impl NextHop {
    pub fn from_gateway(gateway: Option<IpAddress>) -> Self {
        if let Some(gateway) = gateway {
            NextHop::Gateway(gateway)
        } else {
            NextHop::Direct
        }
    }

    pub fn as_gateway(&self) -> Option<IpAddress> {
        match self {
            NextHop::Direct => None,
            NextHop::Gateway(gateway) => Some(*gateway),
        }
    }

    pub fn get(&self, destination: IpAddress) -> IpAddress {
        match self {
            NextHop::Direct => destination,
            NextHop::Gateway(gateway) => *gateway,
        }
    }
}

#[derive(Debug)]
struct Route {
    prefix: IpPrefix,
    next_hop: NextHop,
    iface: Arc<Interface>,
    metric: usize, // lower = preferred
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

    /// Set the given route. If a route already exists for this prefix+iface, it is overwritten.
    pub fn route_set(
        &self,
        prefix: IpPrefix,
        iface: Arc<Interface>,
        gateway: Option<IpAddress>,
        metric: usize,
    ) {
        let new_route = Arc::new(Route {
            prefix,
            next_hop: NextHop::from_gateway(gateway),
            iface,
            metric,
        });

        let mut routes = self.routes.write();

        // Remove the route if existing
        routes.retain(|route| {
            !(route.prefix == new_route.prefix && Arc::ptr_eq(&route.iface, &new_route.iface))
        });

        routes.push(new_route);
    }

    /// Remove a route, given its prefix and interface
    pub fn route_remove(&self, prefix: IpPrefix, iface: &Arc<Interface>) {
        let mut routes = self.routes.write();
        routes.retain(|route| !(route.prefix == prefix && Arc::ptr_eq(&route.iface, iface)));
    }

    /// Test if the given interface is used in the routing table
    pub fn routes_iface_used(&self, iface: &Arc<Interface>) -> bool {
        let routes = self.routes.read();
        routes
            .iter()
            .find(|route| Arc::ptr_eq(&route.iface, iface))
            .is_some()
    }

    /// Remove all routes used by an interface
    pub fn routes_remove_iface(&self, iface: &Arc<Interface>) {
        let mut routes = self.routes.write();
        routes.retain(|route| !Arc::ptr_eq(&route.iface, iface));
    }

    /// List routes
    pub fn routes_list(&self) -> Vec<iface::Route> {
        self.routes
            .read()
            .iter()
            .map(|route| iface::Route {
                prefix: route.prefix,
                gateway: route.next_hop.as_gateway(),
                iface: String::from(route.iface.name()),
                metric: route.metric,
            })
            .collect()
    }

    /// Process an incoming IP packet.
    pub fn receive(&self, iface: &Arc<Interface>, _metadata: EthernetMetadata, packet: Packet) {
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

        if ip_config.ip_address() == dest {
            return true;
        }

        if ip_config.get_prefix().subnet_broadcast() == dest {
            return true;
        }

        false
    }

    fn compute_checksum(&self, header: &mut IpHeader) -> u16 {
        // Backup the original checksum value before setting it to zero for calculation
        let checksum_backup = header.header_checksum;
        header.header_checksum = NetU16::ZERO;

        let mut computer = Checksum::new();
        computer.update(header);

        // Restore the original checksum value
        header.header_checksum = checksum_backup;

        computer.finalize()
    }

    /// Send an IP packet to the specified destination.
    pub async fn send(&self, destination: IpAddress, protocol: u8, mut packet: PacketBuilder) {
        if packet.len() > Ethernet::MTU {
            error!(
                "Packet too big ({}) to fit Ethernet MTU ({}) (dropped)",
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
                .max_by_key(|route| (route.prefix.len(), usize::MAX - route.metric))
                .cloned();

            let Some(route) = route else {
                error!("No route to send packet to {} (dropped)", destination,);
                return;
            };

            route
        };

        let next_hop = route.next_hop.get(destination);

        let source = route
            .iface
            .ip_config()
            .expect("Got interface without IP config")
            .ip_address();

        let mut header = IpHeader {
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

        header.header_checksum = NetU16::from_u16(self.compute_checksum(&mut header));

        packet.prepend(header);

        route.iface.send_ip_packet(next_hop, packet).await;
    }
}
