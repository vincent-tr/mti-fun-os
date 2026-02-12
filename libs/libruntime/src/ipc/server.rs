use super::messages::{
    KHandles, QueryHeader, QueryMessage, ReplyErrorMessage, ReplyHeader, ReplySuccessMessage,
};
use crate::kobject::{self, KObject};
use alloc::{boxed::Box, sync::Arc};
use core::{fmt, marker::PhantomData};
use hashbrown::HashMap;
use log::error;

/// Builder for an IPC server.
pub struct ServerBuilder {
    name: &'static str,
    version: u16,
    handlers: HashMap<u16, Box<dyn MessageHandler>>,
    process_exit_handler: Option<Box<dyn Fn(u64) + 'static>>,
}

impl ServerBuilder {
    /// Creates a new server builder with the given name and version.
    pub fn new(name: &'static str, version: u16) -> Self {
        Self {
            name,
            version,
            handlers: HashMap::new(),
            process_exit_handler: None,
        }
    }

    /// Sets a handler for process exit notifications.
    pub fn with_process_exit_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(u64) + 'static,
    {
        self.process_exit_handler = Some(Box::new(handler));
        self
    }

    /// Adds a message handler without reply for the given message type.
    pub fn with_handler_no_reply<QueryParameters, MessageType, F>(
        mut self,
        message_type: MessageType,
        handler: F,
    ) -> Self
    where
        QueryParameters: Copy + 'static,
        MessageType: Into<u16>,
        F: Fn(QueryParameters, KHandles, u64) + 'static,
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
    pub fn with_handler<QueryParameters, ReplyContent, ReplyError, MessageType, F>(
        mut self,
        message_type: MessageType,
        handler: F,
    ) -> Self
    where
        QueryParameters: Copy + 'static,
        ReplyContent: Copy + 'static,
        ReplyError: Copy + 'static,
        MessageType: Into<u16>,
        F: Fn(QueryParameters, KHandles, u64) -> Result<(ReplyContent, KHandles), ReplyError>
            + 'static,
    {
        self.handlers.insert(
            message_type.into(),
            Box::new(MessageHandlerWithReply::new(handler)),
        );
        self
    }

    /// Builds the server.
    pub fn build(self) -> Result<Server, kobject::Error> {
        Server::new(
            self.name,
            self.version,
            self.handlers,
            self.process_exit_handler,
        )
    }
}

impl fmt::Debug for ServerBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerBuilder")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("handlers", &self.handlers.len())
            .field(
                "process_exit_handler",
                &self.process_exit_handler.as_ref().map(|_| "Fn"),
            )
            .finish()
    }
}

/// Builder for an IPC server, which use a manager pattern
pub struct ManagedServerBuilder<Manager, InternalError, ReplyError>
where
    Manager: 'static,
    InternalError: Into<ReplyError> + 'static,
    ReplyError: Copy + 'static,
{
    builder: ServerBuilder,
    manager: Arc<Manager>,
    _phantom: PhantomData<(InternalError, ReplyError)>,
}

impl<Manager, InternalError, ReplyError> ManagedServerBuilder<Manager, InternalError, ReplyError>
where
    Manager: 'static,
    InternalError: Into<ReplyError> + 'static,
    ReplyError: Copy + 'static,
{
    /// Creates a new server builder with the given manager, name and version.
    pub fn new(manager: &Arc<Manager>, name: &'static str, version: u16) -> Self {
        Self {
            builder: ServerBuilder::new(name, version),
            manager: manager.clone(),
            _phantom: PhantomData,
        }
    }

    /// Sets a handler for process exit notifications, as a server method.
    pub fn with_process_exit_handler<F>(mut self, method: F) -> Self
    where
        F: Fn(&Manager, u64) + 'static,
    {
        let manager = self.manager.clone();
        let handler = move |pid| {
            let instance = manager.clone();
            method(&instance, pid);
        };

        self.builder = self.builder.with_process_exit_handler(handler);
        self
    }

    /// Adds a message handler without reply for the given message type, as a server method.
    pub fn with_handler_no_reply<QueryParameters, MessageType, F>(
        mut self,
        message_type: MessageType,
        method: F,
    ) -> Self
    where
        QueryParameters: Copy + 'static,
        MessageType: Into<u16>,
        F: Fn(&Manager, QueryParameters, KHandles, u64) + 'static,
    {
        let manager = self.manager.clone();
        let handler = move |parameters, handles, sender_pid| {
            let instance = manager.clone();
            method(&instance, parameters, handles, sender_pid);
        };

        self.builder = self.builder.with_handler_no_reply(message_type, handler);
        self
    }

    /// Adds a message handler with reply for the given message type, as a server method.
    pub fn with_handler<QueryParameters, ReplyContent, MessageType, F>(
        mut self,
        message_type: MessageType,
        method: F,
    ) -> Self
    where
        QueryParameters: Copy + 'static,
        ReplyContent: Copy + 'static,
        MessageType: Into<u16>,
        F: Fn(
                &Manager,
                QueryParameters,
                KHandles,
                u64,
            ) -> Result<(ReplyContent, KHandles), InternalError>
            + 'static,
    {
        let manager = self.manager.clone();
        let handler = move |parameters, handles, sender_pid| {
            let instance = manager.clone();
            let res = method(&instance, parameters, handles, sender_pid);
            res.map_err(|e| Into::<ReplyError>::into(e))
        };

        self.builder = self.builder.with_handler(message_type, handler);
        self
    }

    /// Builds the server.
    pub fn build(self) -> Result<Server, kobject::Error> {
        self.builder.build()
    }
}

impl<Manager, InternalError, ReplyError> fmt::Debug
    for ManagedServerBuilder<Manager, InternalError, ReplyError>
where
    Manager: 'static,
    InternalError: Into<ReplyError> + 'static,
    ReplyError: Copy + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.builder.fmt(f)
    }
}

/// IPC server.
///
/// Create using `ServerBuilder`.
#[derive(Debug)]
pub struct Server {
    receiver: kobject::PortReceiver,
    sender: Option<kobject::PortSender>, // Keep sender alive for named port lookup
    process_listener: Option<ProcessTerminationListener>,
    version: u16,
    handlers: HashMap<u16, Box<dyn MessageHandler>>,
}

impl Server {
    fn new(
        name: &str,
        version: u16,
        handlers: HashMap<u16, Box<dyn MessageHandler>>,
        process_exit_handler: Option<Box<dyn Fn(u64) + 'static>>,
    ) -> Result<Self, kobject::Error> {
        let (receiver, sender) = kobject::Port::create(Some(name))?;

        let process_listener = if let Some(handler) = process_exit_handler {
            Some(ProcessTerminationListener::create(handler)?)
        } else {
            None
        };

        Ok(Self {
            receiver,
            sender: Some(sender),
            process_listener,
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
        const RECEIVER_INDEX: usize = 0;
        const PROCESS_LISTENER_INDEX: usize = 1;

        let mut waiter = kobject::Waiter::new(&[&self.receiver]);
        if let Some(process_listener) = &self.process_listener {
            waiter.add(process_listener);
        }

        loop {
            waiter.wait().expect("wait failed");

            if waiter.is_ready(RECEIVER_INDEX) {
                self.process_message();
            }

            if let Some(process_listener) = &self.process_listener {
                if waiter.is_ready(PROCESS_LISTENER_INDEX) {
                    process_listener.process_message();
                }
            }
        }
    }

    fn process_message(&self) {
        let message = self
            .receiver
            .receive()
            .expect("failed to receive message from port");

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

struct ProcessTerminationListener {
    listener: kobject::ProcessListener,
    handler: Box<dyn Fn(u64) + 'static>,
}

impl kobject::KWaitable for ProcessTerminationListener {
    unsafe fn waitable_handle(&self) -> &libsyscalls::Handle {
        self.listener.waitable_handle()
    }

    fn wait(&self) -> Result<(), kobject::Error> {
        self.listener.wait()
    }
}

impl fmt::Debug for ProcessTerminationListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessTerminationListener").finish()
    }
}

impl ProcessTerminationListener {
    pub fn create(handler: Box<dyn Fn(u64) + 'static>) -> Result<Self, kobject::Error> {
        Ok(Self {
            listener: kobject::ProcessListener::create(kobject::ProcessListenerFilter::All)?,
            handler,
        })
    }

    pub fn process_message(&self) {
        let event = self
            .listener
            .receive()
            .expect("failed to receive process event");

        if let kobject::ProcessEventType::Terminated = event.r#type {
            (self.handler)(event.pid);
        }
    }
}

trait MessageHandler: fmt::Debug {
    fn handle_message(&self, message: kobject::Message);
}

struct MessageHandlerWithoutReply<QueryParameters: Copy, F>
where
    F: Fn(QueryParameters, KHandles, u64),
{
    handler: F,
    _phantom: PhantomData<QueryParameters>,
}

impl<QueryParameters: Copy, F> MessageHandlerWithoutReply<QueryParameters, F>
where
    F: Fn(QueryParameters, KHandles, u64),
{
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _phantom: PhantomData,
        }
    }
}

impl<QueryParameters: Copy, F> MessageHandler for MessageHandlerWithoutReply<QueryParameters, F>
where
    F: Fn(QueryParameters, KHandles, u64),
{
    fn handle_message(&self, mut message: kobject::Message) {
        let handles = message.take_all_handles().into();
        let query = unsafe { message.data::<QueryMessage<QueryParameters>>() };
        (self.handler)(query.parameters, handles, query.header.sender_pid);
    }
}

impl<QueryParameters: Copy, F> fmt::Debug for MessageHandlerWithoutReply<QueryParameters, F>
where
    F: Fn(QueryParameters, KHandles, u64),
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageHandlerWithoutReply")
    }
}

// Note: Handler 0 = reply port
struct MessageHandlerWithReply<QueryParameters: Copy, ReplyContent: Copy, ReplyError: Copy, F>
where
    F: Fn(QueryParameters, KHandles, u64) -> Result<(ReplyContent, KHandles), ReplyError>,
{
    handler: F,
    _phantom: (
        PhantomData<QueryParameters>,
        PhantomData<ReplyContent>,
        PhantomData<ReplyError>,
    ),
}

impl<QueryParameters: Copy, ReplyContent: Copy, ReplyError: Copy, F>
    MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError, F>
where
    F: Fn(QueryParameters, KHandles, u64) -> Result<(ReplyContent, KHandles), ReplyError>,
{
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _phantom: (PhantomData, PhantomData, PhantomData),
        }
    }
}

impl<QueryParameters: Copy, ReplyContent: Copy, ReplyError: Copy, F> MessageHandler
    for MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError, F>
where
    F: Fn(QueryParameters, KHandles, u64) -> Result<(ReplyContent, KHandles), ReplyError>,
{
    fn handle_message(&self, mut message: kobject::Message) {
        let port = match kobject::PortSender::from_handle(message.take_handle(0)) {
            Ok(port) => port,
            Err(e) => {
                error!("failed to take reply port handle from message: {:?}", e);
                return;
            }
        };

        let handles = message.take_all_handles().into();
        let query = unsafe { message.data::<QueryMessage<QueryParameters>>() };
        let transaction = query.header.transaction;
        let result = (self.handler)(query.parameters, handles, query.header.sender_pid);

        let mut message = match result {
            Ok((content, handles)) => {
                let reply = ReplySuccessMessage {
                    header: ReplyHeader {
                        transaction,
                        success: true,
                    },
                    content,
                };

                unsafe { kobject::Message::new(&reply, handles.into()) }
            }
            Err(error) => {
                let reply = ReplyErrorMessage {
                    header: ReplyHeader {
                        transaction,
                        success: false,
                    },
                    error,
                };

                unsafe { kobject::Message::new(&reply, KHandles::new().into()) }
            }
        };

        if let Err(e) = port.send(&mut message) {
            error!("failed to send success reply: {:?}", e);
        }
    }
}

impl<QueryParameters: Copy, ReplyContent: Copy, ReplyError: Copy, F> fmt::Debug
    for MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError, F>
where
    F: Fn(QueryParameters, KHandles, u64) -> Result<(ReplyContent, KHandles), ReplyError>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageHandlerWithReply")
    }
}
