mod arp;
mod ethernet;
mod int;

use int::*;

use arp::Arp;
use ethernet::{Ethernet, EthernetMetadata};

use crate::{iface::Interface, packet::Packet};

/// Stack of protocols using by an interface
#[derive(Debug)]
pub struct InterfaceProtocols {
    ethernet: Ethernet,
    arp: Arp,
}

impl InterfaceProtocols {
    /// Create a new protocol stack for an interface.
    pub fn new() -> Self {
        Self {
            ethernet: Ethernet::new(),
            arp: Arp::new(),
        }
    }

    /// Get a reference to the Ethernet protocol instance.
    pub fn ethernet(&self) -> &Ethernet {
        &self.ethernet
    }

    /// Get a reference to the ARP protocol instance.
    pub fn arp(&self) -> &Arp {
        &self.arp
    }

    /// Process an incoming packet on the interface, given the raw bytes of the packet.
    pub async fn receive(&self, iface: &Interface, packet: Packet) {
        self.ethernet.receive(iface, packet).await;
    }
}
