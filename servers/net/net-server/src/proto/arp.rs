use core::{fmt, mem};

use alloc::{sync::Arc, vec::Vec};
use hashbrown::HashMap;
use libruntime::{
    r#async,
    net::types::{IpAddress, MacAddress},
    sync::{Mutex, r#async::NotifyOnce},
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

/// Errors that can occur during ARP resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArpError {
    Timeout,
    Canceled,
}

impl fmt::Display for ArpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArpError::Timeout => write!(f, "ARP resolution timed out"),
            ArpError::Canceled => write!(f, "ARP resolution was canceled"),
        }
    }
}

/// A helper struct to manage the completion of an ARP resolution, allowing multiple waiters to await the result of a single resolution attempt.
#[derive(Debug, Clone)]
struct ResolutionCompletion {
    result: Arc<Mutex<Option<Result<MacAddress, ArpError>>>>,
    completion: NotifyOnce,
}

impl ResolutionCompletion {
    /// Create a new resolution completion that can be awaited for an ARP resolution result.
    pub fn new() -> Self {
        Self {
            result: Arc::new(Mutex::new(None)),
            completion: NotifyOnce::new(),
        }
    }

    /// Wait for this resolution to complete and return the result.
    pub async fn wait(&self) -> Result<MacAddress, ArpError> {
        self.completion.wait().await;

        self.result
            .lock()
            .clone()
            .expect("completion should have a result")
    }

    /// Complete this resolution with the given result, waking any waiters.
    pub fn complete(&self, result: Result<MacAddress, ArpError>) {
        let mut locked_result = self.result.lock();
        assert!(
            locked_result.is_none(),
            "completion should only be completed once"
        );
        *locked_result = Some(result);

        self.completion.notify();
    }
}

#[derive(Debug)]
enum CacheEntry {
    Pending(PendingEntry),
    Resolved(ResolvedEntry),
}

#[derive(Debug)]
struct PendingEntry {
    completion: ResolutionCompletion,
    timeout_at: time::Duration,
    retries: usize,
}

#[derive(Debug)]
struct ResolvedEntry {
    mac: MacAddress,
    expires_at: time::Duration,
}

/// ARP protocol implementation.
#[derive(Debug)]
pub struct Arp {
    iface: Arc<Interface>,
    cache: Mutex<HashMap<IpAddress, CacheEntry>>,
}

impl Arp {
    /// Timeout duration for ARP cache entries before they expire and need to be refreshed.
    const EXPIRE_TIMEOUT: time::Duration = time::Duration::seconds(60);

    /// Timeout duration for ARP queries.
    const QUERY_TIMEOUT: time::Duration = time::Duration::seconds(5);

    /// Number of retries to attempt for ARP queries before giving up and returning a timeout error.
    const QUERY_RETRIES: usize = 3;

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

    /// Resolve the given IP address to a MAC address, performing an ARP request if necessary.
    pub async fn resolve(&self, ip: IpAddress) -> Result<MacAddress, ArpError> {
        let completion = {
            let mut cache = self.cache.lock();

            let entry = cache.entry(ip).or_insert_with(|| {
                debug!("[{}] Starting ARP resolution for {}", self.name(), ip);

                let pending = PendingEntry {
                    completion: ResolutionCompletion::new(),
                    timeout_at: time::get_monotonic_time() + Self::QUERY_TIMEOUT,
                    retries: 0,
                };

                self.start_resolution(ip);

                CacheEntry::Pending(pending)
            });

            match entry {
                CacheEntry::Resolved(resolved) => {
                    return Ok(resolved.mac);
                }
                CacheEntry::Pending(pending) => pending.completion.clone(),
            }
        };

        completion.wait().await
    }

    /// Perform periodic maintenance tasks.
    pub fn tick(&self) {
        let now = time::get_monotonic_time();
        let mut cache = self.cache.lock();

        // Process retry timeouts
        let mut remove_list = Vec::new();

        for (&ip, entry) in cache.iter_mut() {
            match entry {
                CacheEntry::Pending(pending) => {
                    if pending.timeout_at <= now {
                        if pending.retries >= Self::QUERY_RETRIES {
                            debug!(
                                "[{}] ARP resolution for {} timed out after {} retries",
                                self.name(),
                                ip,
                                pending.retries
                            );

                            pending.completion.complete(Err(ArpError::Timeout));
                            remove_list.push(ip);
                        } else {
                            debug!(
                                "[{}] Retrying ARP resolution for {} (retry {}/{})",
                                self.name(),
                                ip,
                                pending.retries + 1,
                                Self::QUERY_RETRIES
                            );

                            self.start_resolution(ip);
                            pending.timeout_at = now + Self::QUERY_TIMEOUT;
                            pending.retries += 1;
                        }
                    }
                }
                CacheEntry::Resolved(resolved) => {
                    if resolved.expires_at <= now {
                        debug!("[{}] ARP cache entry for {} expired", self.name(), ip);

                        remove_list.push(ip);
                    }
                }
            }
        }

        for ip in remove_list {
            cache.remove(&ip);
        }
    }

    /// Clean up any resources used by this ARP instance. After calling this method, the instance should not be used anymore.
    pub fn destroy(&self) {
        let mut cache = self.cache.lock();

        for entry in cache.values_mut() {
            if let CacheEntry::Pending(pending) = entry {
                pending.completion.complete(Err(ArpError::Canceled));
            }
        }

        cache.clear();
    }

    /// Update the ARP cache with a resolved IP-to-MAC mapping.
    pub fn update(&self, ip: IpAddress, mac: MacAddress) {
        debug!("[{}] Updating ARP cache: {} is at {}", self.name(), ip, mac);
        let mut cache = self.cache.lock();

        let resolved = ResolvedEntry {
            mac,
            expires_at: time::get_monotonic_time() + Self::EXPIRE_TIMEOUT,
        };

        if let Some(CacheEntry::Pending(pending)) = cache.insert(ip, CacheEntry::Resolved(resolved))
        {
            pending.completion.complete(Ok(mac));
        }
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

    fn start_resolution(&self, ip: IpAddress) {
        let iface = self.iface().clone();
        r#async::spawn(async move { iface.protocols().arp.send_request(ip).await });
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
        builder.prepend(arp_reply);

        self.iface()
            .protocols()
            .ethernet()
            .send(target_mac, Ethernet::ARP, builder)
            .await;
    }

    async fn send_request(&self, target_ip: IpAddress) {
        let mut builder = PacketBuilder::new();

        let arp_request = ArpPacket {
            htype: NetU16::from_u16(Self::HTYPE_ETHERNET),
            ptype: NetU16::from_u16(Ethernet::IPV4),
            hlen: 6,
            plen: 4,
            oper: NetU16::from_u16(Self::OPER_REQUEST),
            sha: self.iface().mac_address(),
            spa: self.iface().ip_config().expect("no ip config").ip_address(),
            tha: MacAddress::broadcast(),
            tpa: target_ip,
        };
        builder.prepend(arp_request);

        self.iface()
            .protocols()
            .ethernet()
            .send(MacAddress::broadcast(), Ethernet::ARP, builder)
            .await;
    }
}
