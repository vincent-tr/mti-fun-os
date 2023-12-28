use alloc::{collections::LinkedList, string::String, sync::Arc};
use spin::RwLock;
use syscalls::Error;

use crate::user::{
    handle::{Handle, KernelHandle},
    process::Process,
    thread::{ThreadError, WaitQueue},
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
    message_queue: RwLock<LinkedList<InternalMessage>>,
    reader_queue: WaitQueue,
}

impl Port {
    fn new(id: u64, name: &str) -> Arc<Self> {
        Arc::new(Self {
            id,
            name: String::from(name),
            message_queue: RwLock::new(LinkedList::new()),
            reader_queue: WaitQueue::new(),
        })
    }

    /// Send a message to the port
    pub fn send(&self, sender: &Arc<Process>, message: Message) -> Result<(), Error> {
        let message = InternalMessage::from(sender, &message)?;

        let mut message_queue = self.message_queue.write();
        message_queue.push_back(message);

        Ok(())
    }

    /// Receive a message from the port
    ///
    /// Note: the operation does not block
    pub fn receive(&self, receiver: &Arc<Process>) -> Option<Message> {
        let mut message_queue = self.message_queue.write();

        if let Some(message) = message_queue.pop_front() {
            Some(message.to(receiver))
        } else {
            None
        }
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

