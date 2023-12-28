use alloc::sync::Arc;
use syscalls::Error;

use crate::user::process::Process;

use super::{Message, Port};

pub fn access(port: Arc<Port>) -> (Arc<PortReceiver>, Arc<PortSender>) {
    (PortReceiver::new(port.clone()), PortSender::new(port))
}

#[derive(Debug)]
pub struct PortReceiver {
    port: Arc<Port>,
}

impl PortReceiver {
    fn new(port: Arc<Port>) -> Arc<Self> {
        Arc::new(Self { port })
    }

    /// Get the port identifier
    pub fn id(&self) -> u64 {
        self.port.id()
    }

    /// Get the port name
    pub fn name<'a>(&'a self) -> &'a str {
        &self.port.name()
    }

    /// Receive a message from the port
    ///
    /// Note: the operation does not block
    pub fn receive(&self, receiver: &Arc<Process>) -> Option<Message> {
        self.port.receive(receiver)
    }
}

impl Drop for PortReceiver {
    fn drop(&mut self) {
        self.port.close();
    }
}

#[derive(Debug)]
pub struct PortSender {
    port: Arc<Port>,
}

impl PortSender {
    fn new(port: Arc<Port>) -> Arc<Self> {
        Arc::new(Self { port })
    }

    /// Get the port identifier
    pub fn id(&self) -> u64 {
        self.port.id()
    }

    /// Get the port name
    pub fn name<'a>(&'a self) -> &'a str {
        &self.port.name()
    }

    /// Send a message to the port
    pub fn send(&self, sender: &Arc<Process>, message: Message) -> Result<(), Error> {
        self.port.send(sender, message)
    }
}
