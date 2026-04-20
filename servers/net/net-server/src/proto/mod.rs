mod arp;
mod ethernet;
mod int;

use alloc::sync::Arc;
use int::*;

use arp::Arp;
use ethernet::{Ethernet, EthernetMetadata};
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
