use alloc::{collections::LinkedList, string::String, sync::Arc};
use spin::RwLock;
use syscalls::Error;

use crate::user::{
    error::port_closed,
    handle::{Handle, KernelHandle},
    process::Process,
    thread::WaitQueue,
};

use super::Message;

/// Standalone function, so that Port::new() can remain private
///
/// Note: Only Port type is exported by port module, not this function
pub fn new(id: u64, name: &str) -> Arc<Port> {
    Port::new(id, name)
}

/// Port: implementation of a mailbox
///
/// Several senders can send messages to a port
///
/// One receiver can get them. (Multiple can get them to balance load, but each message will only be distributed to one receiver)
#[derive(Debug)]
pub struct Port {
    id: u64,
    name: String,
    data: RwLock<Data>,
    reader_queue: WaitQueue,
}

#[derive(Debug)]
struct Data {
    message_queue: LinkedList<InternalMessage>,
    closed: bool,
}

impl Port {
    fn new(id: u64, name: &str) -> Arc<Self> {
        Arc::new(Self {
            id,
            name: String::from(name),
            data: RwLock::new(Data {
                message_queue: LinkedList::new(),
                closed: false,
            }),
            reader_queue: WaitQueue::new(),
        })
    }

    /// Get the port identifier
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the port name
    pub fn name<'a>(&'a self) -> &'a str {
        &self.name
    }

    /// Send a message to the port
    pub fn send(&self, sender: &Arc<Process>, message: Message) -> Result<(), Error> {
        let mut data = self.data.write();
        if data.closed {
            return Err(port_closed());
        }

        let message = InternalMessage::from(sender, &message)?;

        data.message_queue.push_back(message);

        // TODO: wake up

        Ok(())
    }

    /// Receive a message from the port
    ///
    /// Note: the operation does not block
    pub fn receive(&self, receiver: &Arc<Process>) -> Option<Message> {
        let mut data = self.data.write();
        assert!(!data.closed);

        if let Some(message) = data.message_queue.pop_front() {
            Some(message.to(receiver))
        } else {
            None
        }
    }

    /// Called when receiver is dropped: No one will ever be able to read the messages, so drop them
    pub fn close(&self) {
        let mut data = self.data.write();
        assert!(!data.closed);

        data.closed = true;
        data.message_queue.clear();

        // TODO: wake up
    }

    pub fn closed(&self) -> bool {
        let data = self.data.read();
        data.closed
    }

    pub fn message_queue_count(&self) -> usize {
        let data = self.data.read();
        data.message_queue.len()
    }
}

#[derive(Debug)]
struct InternalMessage {
    data: [u64; Message::DATA_SIZE],
    handles: [Option<KernelHandle>; Message::HANDLE_COUNT],
}

impl InternalMessage {
    pub fn from(sender: &Arc<Process>, message: &Message) -> Result<Self, Error> {
        const NO_HANDLE: Option<KernelHandle> = None;

        let mut internal_message = InternalMessage {
            data: message.data,
            handles: [NO_HANDLE; Message::HANDLE_COUNT],
        };

        for index in 0..Message::HANDLE_COUNT {
            let handle = message.handles[index];
            if handle.valid() {
                internal_message.handles[index] = Some(sender.handles().get(handle)?);
            }
        }

        // Now that we could get all handle successfully, close them on the sender
        for index in 0..Message::HANDLE_COUNT {
            let handle = message.handles[index];
            if handle.valid() {
                sender
                    .handles()
                    .close(handle)
                    .expect("Could not close handle");
            }
        }

        Ok(internal_message)
    }

    pub fn to(self, receiver: &Arc<Process>) -> Message {
        // Create handles in the receiver
        const NO_HANDLE: Handle = Handle::invalid();

        let mut message = Message {
            data: self.data,
            handles: [NO_HANDLE; Message::HANDLE_COUNT],
        };

        for index in 0..Message::HANDLE_COUNT {
            if let Some(kernel_handle) = &self.handles[index] {
                message.handles[index] = receiver.handles().open(kernel_handle.clone());
            }
        }

        message
    }
}
