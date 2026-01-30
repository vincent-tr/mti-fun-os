use super::messages::{
    Handles, QueryHeader, QueryMessage, ReplyErrorMessage, ReplyHeader, ReplySuccessMessage,
};
use crate::kobject::{self, KObject};
use alloc::boxed::Box;
use hashbrown::HashMap;
use log::error;

/// Builder for an IPC server.
#[derive(Debug)]
pub struct ServerBuilder {
    name: &'static str,
    version: u16,
    handlers: HashMap<u16, Box<dyn MessageHandler>>,
}

impl ServerBuilder {
    /// Creates a new server builder with the given name and version.
    pub fn new(name: &'static str, version: u16) -> Self {
        Self {
            name,
            version,
            handlers: HashMap::new(),
        }
    }

    /// Adds a message handler without reply for the given message type.
    pub fn with_handler_no_reply<QueryParameters, MessageType>(
        mut self,
        message_type: MessageType,
        handler: fn(QueryParameters, Handles),
    ) -> Self
    where
        QueryParameters: Copy + 'static,
        MessageType: Into<u16>,
    {
        self.handlers.insert(
            message_type.into(),
            Box::new(MessageHandlerWithoutReply::new(handler)),
        );
        self
    }

    /// Adds a message handler with reply for the given message type.
    ///
    /// Note: handles[0] is reserved for the reply port, and will be set to invalid before calling the handler.
    pub fn with_handler<QueryParameters, ReplyContent, ReplyError, MessageType>(
        mut self,
        message_type: MessageType,
        handler: fn(QueryParameters, Handles) -> Result<(ReplyContent, Handles), ReplyError>,
    ) -> Self
    where
        QueryParameters: Copy + 'static,
        ReplyContent: Copy + 'static,
        ReplyError: Copy + 'static,
        MessageType: Into<u16>,
    {
        self.handlers.insert(
            message_type.into(),
            Box::new(MessageHandlerWithReply::new(handler)),
        );
        self
    }

    /// Builds the server.
    pub fn build(self) -> Result<Server, kobject::Error> {
        Server::new(self.name, self.version, self.handlers)
    }
}

/// IPC server.
///
/// Create using `ServerBuilder`.
#[derive(Debug)]
pub struct Server {
    receiver: kobject::PortReceiver,
    sender: Option<kobject::PortSender>, // Keep sender alive for named port lookup
    version: u16,
    handlers: HashMap<u16, Box<dyn MessageHandler>>,
}

impl Server {
    fn new(
        name: &str,
        version: u16,
        handlers: HashMap<u16, Box<dyn MessageHandler>>,
    ) -> Result<Self, kobject::Error> {
        let (receiver, sender) = kobject::Port::create(Some(name))?;

        Ok(Self {
            receiver,
            sender: Some(sender),
            version,
            handlers,
        })
    }

    /// Releases the name of the server port.
    ///
    /// The server will still respond to already "connected" clients, but will be unreachable for new lookups.
    pub fn release_name(&mut self) {
        self.sender = None;
    }

    /// Runs the server.
    pub fn run(self) -> ! {
        loop {
            let message = self
                .receiver
                .blocking_receive()
                .expect("failed to receive message from port");

            self.process_message(message);
        }
    }

    fn process_message(&self, message: kobject::Message) {
        let header = unsafe { message.data::<QueryHeader>() };
        if header.version != self.version {
            error!("invalid message version: {}", header.version);
            return;
        }

        let Some(handler) = self.handlers.get(&header.r#type) else {
            error!("no handler for message type: {}", header.r#type);
            return;
        };

        handler.handle_message(message);
    }
}

trait MessageHandler: core::fmt::Debug {
    fn handle_message(&self, message: kobject::Message);
}

struct MessageHandlerWithoutReply<QueryParameters: Copy> {
    handler: fn(QueryParameters, Handles),
}

impl<QueryParameters: Copy> MessageHandlerWithoutReply<QueryParameters> {
    pub fn new(handler: fn(QueryParameters, Handles)) -> Self {
        Self { handler }
    }
}

impl<QueryParameters: Copy> MessageHandler for MessageHandlerWithoutReply<QueryParameters> {
    fn handle_message(&self, mut message: kobject::Message) {
        let query = unsafe { message.data::<QueryMessage<QueryParameters>>() };
        (self.handler)(query.parameters, message.take_all_handles());
    }
}

impl<QueryParameters: Copy> core::fmt::Debug for MessageHandlerWithoutReply<QueryParameters> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "MessageHandlerWithoutReply")
    }
}

// Note: Handler 0 = reply port
struct MessageHandlerWithReply<QueryParameters: Copy, ReplyContent: Copy, ReplyError: Copy> {
    handler: fn(QueryParameters, Handles) -> Result<(ReplyContent, Handles), ReplyError>,
}

impl<QueryParameters: Copy, ReplyContent: Copy, ReplyError: Copy>
    MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError>
{
    pub fn new(
        handler: fn(QueryParameters, Handles) -> Result<(ReplyContent, Handles), ReplyError>,
    ) -> Self {
        Self { handler }
    }
}

impl<QueryParameters: Copy, ReplyContent: Copy, ReplyError: Copy> MessageHandler
    for MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError>
{
    fn handle_message(&self, mut message: kobject::Message) {
        let port = match kobject::PortSender::from_handle(message.take_handle(0)) {
            Ok(port) => port,
            Err(e) => {
                error!("failed to take reply port handle from message: {:?}", e);
                return;
            }
        };

        let query = unsafe { message.data::<QueryMessage<QueryParameters>>() };
        let transaction = query.header.transaction;
        let result = (self.handler)(query.parameters, message.take_all_handles());

        let mut message = match result {
            Ok((content, mut handles)) => {
                let reply = ReplySuccessMessage {
                    header: ReplyHeader {
                        transaction,
                        success: true,
                    },
                    content,
                };

                unsafe { kobject::Message::new(&reply, handles.as_mut_slice()) }
            }
            Err(error) => {
                let reply = ReplyErrorMessage {
                    header: ReplyHeader {
                        transaction,
                        success: false,
                    },
                    error,
                };

                unsafe { kobject::Message::new(&reply, &mut []) }
            }
        };

        if let Err(e) = port.send(&mut message) {
            error!("failed to send success reply: {:?}", e);
        }
    }
}

impl<QueryParameters: Copy, ReplyContent: Copy, ReplyError: Copy> core::fmt::Debug
    for MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "MessageHandlerWithReply")
    }
}
