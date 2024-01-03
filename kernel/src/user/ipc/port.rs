use alloc::{collections::LinkedList, string::String, sync::Arc};
use spin::RwLock;
use syscalls::{Error, Message};

use crate::user::{
    error::{object_closed, object_not_ready},
    handle::{Handle, KernelHandle},
    process::Process,
    thread::{self, WaitQueue},
};

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
    receiver_queue: Arc<WaitQueue>,
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
            receiver_queue: Arc::new(WaitQueue::new()),
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
            return Err(object_closed());
        }

        let message = InternalMessage::from(sender, &message)?;
        data.message_queue.push_back(message);

        // Wake up any waiting receiver
        thread::wait_queue_wake_all(&self.receiver_queue);

        Ok(())
    }

    /// Receive a message from the port
    ///
    /// Note: the operation does not block, and return Error::ObjectNotReady if there is no message available
    pub fn receive(&self, receiver: &Arc<Process>) -> Result<Message, Error> {
        let mut data = self.data.write();
        // Should not be able to receive on closed port since there is no receiver anymore
        assert!(!data.closed);

        if let Some(message) = data.message_queue.pop_front() {
            Ok(message.to(receiver))
        } else {
            Err(object_not_ready())
        }
    }

    /// Called when receiver is dropped: No one will ever be able to read the messages, so drop them
    pub fn close(&self) {
        let mut data = self.data.write();
        // Should not be able to close on closed port since there is already no receiver anymore
        assert!(!data.closed);

        data.closed = true;
        data.message_queue.clear();

        // Wait up any sleeping receivers (They won't be able to receive)
        thread::wait_queue_wake_all(&self.receiver_queue);
    }

    /// Prepare a wait
    ///
    /// Return None if the port is already ready for receive
    pub fn prepare_wait(&self) -> Option<&Arc<WaitQueue>> {
        let data = self.data.read();
        // Should not be able to wait on closed port since there is no receiver anymore
        assert!(!data.closed);

        if data.message_queue.is_empty() {
            Some(&self.receiver_queue)
        } else {
            None
        }
    }

    pub fn closed(&self) -> bool {
        let data = self.data.read();
        data.closed
    }

    pub fn message_queue_count(&self) -> usize {
        let data = self.data.read();
        data.message_queue.len()
    }

    pub fn waiting_receiver_count(&self) -> usize {
        self.receiver_queue.len()
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
            let handle = Handle::from(message.handles[index]);
            if handle.valid() {
                internal_message.handles[index] = Some(sender.handles().get(handle)?);
            }
        }

        // Now that we could get all handle successfully, close them on the sender
        for index in 0..Message::HANDLE_COUNT {
            let handle = Handle::from(message.handles[index]);
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
        const NO_HANDLE: u64 = Handle::invalid().as_u64();

        let mut message = Message {
            data: self.data,
            handles: [NO_HANDLE; Message::HANDLE_COUNT],
        };

        for index in 0..Message::HANDLE_COUNT {
            if let Some(kernel_handle) = &self.handles[index] {
                message.handles[index] = receiver.handles().open(kernel_handle.clone()).as_u64();
            }
        }

        message
    }
}
