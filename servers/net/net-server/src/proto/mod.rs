mod arp;
mod ethernet;
mod int;

use alloc::sync::Arc;
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
    pub fn new(iface: &Arc<Interface>) -> Self {
        Self {
            ethernet: Ethernet::new(iface.clone()),
            arp: Arp::new(iface.clone()),
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

    /// Perform periodic maintenance tasks for all protocols in the stack.
    pub fn tick(&self) {
        self.arp.tick();
    }

    /// Process an incoming packet on the interface, given the raw bytes of the packet.
    pub async fn receive(&self, packet: Packet) {
        self.ethernet.receive(packet).await;
    }
}
