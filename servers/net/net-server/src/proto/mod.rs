mod arp;
mod checksum;
mod ethernet;
mod icmp;
mod int;
mod ip;
mod tcp;
mod udp;

use alloc::sync::Arc;
use checksum::*;
use int::*;

use arp::Arp;
use ethernet::{Ethernet, EthernetMetadata};
use icmp::Icmp;
use ip::{Ip, IpMetadata};
use tcp::Tcp;
use udp::Udp;

use lazy_static::lazy_static;
use libruntime::net::types::{IpAddress, MacAddress};
use log::error;

use crate::{
    iface::Interface,
    packet::{Packet, PacketBuilder},
};

/// Stack of protocols using by an interface
#[derive(Debug)]
pub struct InterfaceProtocols {
    iface: Arc<Interface>,
    ethernet: Ethernet,
    arp: Arp,
}

impl InterfaceProtocols {
    /// Create a new protocol stack for an interface.
    pub fn new(iface: &Arc<Interface>) -> Self {
        Self {
            iface: iface.clone(),
            ethernet: Ethernet::new(iface.clone()),
            arp: Arp::new(iface.clone()),
        }
    }

    fn iface(&self) -> &Interface {
        &self.iface
    }

    fn name(&self) -> &str {
        self.iface().name()
    }

    /// Get a reference to the Ethernet protocol instance.
    pub fn ethernet(&self) -> &Ethernet {
        &self.ethernet
    }

    /// Get a reference to the ARP protocol instance.
    pub fn arp(&self) -> &Arp {
        &self.arp
    }

    /// Perform periodic maintenance tasks for all protocols in the stack.
    pub fn tick(&self) {
        self.arp.tick();
    }

    /// Destroy all protocols in the stack, performing any necessary cleanup.
    pub fn destroy(&self) {
        self.arp.destroy();
    }

    /// Process an incoming packet on the interface, given the raw bytes of the packet.
    pub fn receive_ethernet_frame(&self, packet: Packet) {
        self.ethernet.receive(packet);
    }

    /// Send an IP packet to the specified next hop IP address.
    pub async fn send_ip_packet(&self, next_hop: IpAddress, packet: PacketBuilder) {
        let mac = if next_hop.is_broadcast() {
            MacAddress::broadcast()
        } else {
            match self.arp().resolve(next_hop).await {
                Ok(mac) => mac,
                Err(e) => {
                    error!(
                        "[{}] Failed to resolve IP address {} to MAC address (packet dropped): {}",
                        self.name(),
                        next_hop,
                        e
                    );
                    return;
                }
            }
        };

        self.ethernet().send(mac, Ethernet::IPV4, packet).await;
    }
}

/// Global protocol instances that are shared across all interfaces.
#[derive(Debug)]
pub struct GlobalProtocols {
    ip: Ip,
    icmp: Icmp,
    tcp: Tcp,
    udp: Udp,
}

impl GlobalProtocols {
    /// Create new global protocol instances.
    fn new() -> Self {
        Self {
            ip: Ip::new(),
            icmp: Icmp::new(),
            tcp: Tcp::new(),
            udp: Udp::new(),
        }
    }

    /// Get a reference to the global protocol instance.
    pub fn instance() -> &'static Self {
        lazy_static! {
            static ref INSTANCE: GlobalProtocols = GlobalProtocols::new();
        }

        &INSTANCE
    }

    /// Get a reference to the IP protocol instance.
    pub fn ip(&self) -> &Ip {
        &self.ip
    }

    /// Get a reference to the ICMP protocol instance.
    pub fn icmp(&self) -> &Icmp {
        &self.icmp
    }

    /// Get a reference to the TCP protocol instance.
    pub fn tcp(&self) -> &Tcp {
        &self.tcp
    }

    /// Get a reference to the UDP protocol instance.
    pub fn udp(&self) -> &Udp {
        &self.udp
    }
}
