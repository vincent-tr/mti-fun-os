use core::mem;

use alloc::{sync::Arc, vec::Vec};
use hashbrown::HashMap;
use libruntime::{
    r#async,
    net::types::{IpAddress, MacAddress},
    sync::Mutex,
    time,
};
use log::{debug, warn};

use crate::{
    iface::Interface,
    packet::{Packet, PacketBuilder, PacketCursor},
};

use super::*;

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

#[derive(Debug)]
enum CacheEntry {
    Pending {
        pending: Vec<Packet>,
        sent_at: time::Duration,
        retries: usize,
    },
    Resolved {
        mac: MacAddress,
        expires_at: time::Duration,
    },
}

/// ARP protocol implementation.
#[derive(Debug)]
pub struct Arp {
    iface: Arc<Interface>,
    cache: Mutex<HashMap<IpAddress, CacheEntry>>,
}

impl Arp {
    /// Timeout duration for ARP cache entries before they expire and need to be refreshed.
    pub const EXPIRE_TIMEOUT: time::Duration = time::Duration::seconds(60);

    /// Hardware type value for Ethernet in ARP packets.
    const HTYPE_ETHERNET: u16 = 1;

    /// Operation type value for ARP request packets.
    const OPER_REQUEST: u16 = 1;

    /// Operation type value for ARP reply packets.
    const OPER_REPLY: u16 = 2;

    /// Create a new ARP protocol instance.
    pub fn new(iface: Arc<Interface>) -> Self {
        Self {
            iface,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn iface(&self) -> &Arc<Interface> {
        &self.iface
    }

    fn name(&self) -> &str {
        self.iface().name()
    }

    /// Perform periodic maintenance tasks.
    pub fn tick(&self) {
        let now = time::get_monotonic_time();
        let mut cache = self.cache.lock();

        cache.retain(|_, entry| match entry {
            CacheEntry::Pending {
                sent_at, retries, ..
            } => {
                // For pending entries, we can implement retry logic here if desired. For now, we will just keep them indefinitely.
                true
            }
            CacheEntry::Resolved { expires_at, .. } => {
                // Remove resolved entries that have expired.
                if *expires_at > now {
                    true
                } else {
                    debug!("[{}] ARP cache entry expired and removed", self.name());
                    false
                }
            }
        });
    }

    /// Update the ARP cache with a resolved IP-to-MAC mapping.
    fn update(&self, ip: IpAddress, mac: MacAddress) {
        debug!("[{}] Updating ARP cache: {} is at {}", self.name(), ip, mac);
        let mut cache = self.cache.lock();

        // TODO: If there is a pending entry for this IP, we should send all pending packets to the resolved MAC address.

        cache.insert(
            ip,
            CacheEntry::Resolved {
                mac,
                expires_at: time::get_monotonic_time() + Self::EXPIRE_TIMEOUT,
            },
        );
    }

    /// Process an incoming ARP packet.
    pub fn receive(&self, metadata: EthernetMetadata, packet: Packet) {
        if packet.len() < mem::size_of::<ArpPacket>() {
            warn!(
                "[{}] Received packet too short to contain ARP header: length={}, from {} (dropped)",
                self.name(),
                packet.len(),
                metadata.source
            );
            return;
        }

        let mut cursor = PacketCursor::new(&packet);
        let arp_packet = cursor
            .read::<ArpPacket>()
            .expect("Could not read ARP packet");

        if arp_packet.htype.to_u16() != Self::HTYPE_ETHERNET
            || arp_packet.ptype.to_u16() != Ethernet::IPV4
            || arp_packet.hlen != 6
            || arp_packet.plen != 4
        {
            debug!(
                "[{}] Received ARP packet with unsupported type or lengths: htype={}, ptype={}, hlen={}, plen={} (dropped)",
                self.name(),
                arp_packet.htype.to_u16(),
                arp_packet.ptype.to_u16(),
                arp_packet.hlen,
                arp_packet.plen
            );
            return;
        }

        match arp_packet.oper.to_u16() {
            Self::OPER_REQUEST => {
                self.update(arp_packet.spa, arp_packet.sha);

                if let Some(ip_config) = self.iface().ip_config()
                    && arp_packet.tpa == ip_config.ip_address()
                {
                    let iface = self.iface().clone();
                    r#async::spawn(async move {
                        iface
                            .protocols()
                            .arp
                            .send_reply(arp_packet.spa, arp_packet.sha)
                            .await
                    });
                }
            }
            Self::OPER_REPLY => {
                self.update(arp_packet.spa, arp_packet.sha);
                self.update(arp_packet.tpa, arp_packet.tha);
            }
            _ => {
                debug!(
                    "[{}] Received ARP packet with unknown operation: oper={} from {} (dropped)",
                    self.name(),
                    arp_packet.oper.to_u16(),
                    metadata.source,
                );
                return;
            }
        }
    }

    async fn send_reply(&self, target_ip: IpAddress, target_mac: MacAddress) {
        let mut builder = PacketBuilder::new();

        let arp_reply = ArpPacket {
            htype: NetU16::from_u16(Self::HTYPE_ETHERNET),
            ptype: NetU16::from_u16(Ethernet::IPV4),
            hlen: 6,
            plen: 4,
            oper: NetU16::from_u16(Self::OPER_REPLY),
            sha: self.iface().mac_address(),
            spa: self.iface().ip_config().expect("no ip config").ip_address(),
            tha: target_mac,
            tpa: target_ip,
        };
        builder.prepend(&arp_reply);

        self.iface()
            .protocols()
            .ethernet()
            .send(target_mac, Ethernet::ARP, builder)
            .await;
    }
}
