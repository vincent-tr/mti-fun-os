use super::messages::{
    KHandles, QueryHeader, QueryMessage, ReplyErrorMessage, ReplyHeader, ReplySuccessMessage,
};
use crate::{
    kobject::{self, KObject},
    r#async,
};
use alloc::{boxed::Box, sync::Arc};
use core::{fmt, future::Future, marker::PhantomData};
use hashbrown::HashMap;
use log::error;

/// Builder for an async IPC server.
pub struct AsyncServerBuilder {
    name: &'static str,
    version: u16,
    handlers: HashMap<u16, Box<dyn MessageHandler>>,
    process_exit_handler: Option<Box<dyn ProcessTerminationHandler>>,
}

impl AsyncServerBuilder {
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
    pub fn with_process_exit_handler<Fut, F>(mut self, handler: F) -> Self
    where
        Fut: Future<Output = ()> + Send + 'static,
        F: Fn(u64) -> Fut + Sync + Send + 'static,
    {
        self.process_exit_handler = Some(Box::new(ProcessTerminationHandlerImpl::new(handler)));
        self
    }

    /// Adds a message handler without reply for the given message type.
    pub fn with_handler_no_reply<QueryParameters, MessageType, Fut, F>(
        mut self,
        message_type: MessageType,
        handler: F,
    ) -> Self
    where
        QueryParameters: Copy + Sync + Send + 'static,
        MessageType: Into<u16>,
        Fut: Future<Output = ()> + Sync + Send + 'static,
        F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
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
    pub fn with_handler<QueryParameters, ReplyContent, ReplyError, MessageType, Fut, F>(
        mut self,
        message_type: MessageType,
        handler: F,
    ) -> Self
    where
        QueryParameters: Copy + Sync + Send + 'static,
        ReplyContent: Copy + Sync + Send + 'static,
        ReplyError: Copy + Sync + Send + 'static,
        MessageType: Into<u16>,
        Fut: Future<Output = Result<(ReplyContent, KHandles), ReplyError>> + Send + 'static,
        F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
    {
        self.handlers.insert(
            message_type.into(),
            Box::new(MessageHandlerWithReply::new(handler)),
        );
        self
    }

    /// Builds the server.
    pub fn build(self) -> Result<AsyncServer, kobject::Error> {
        AsyncServer::new(
            self.name,
            self.version,
            self.handlers,
            self.process_exit_handler,
        )
    }
}

impl fmt::Debug for AsyncServerBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncServerBuilder")
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

/// Builder for an async IPC server, which uses a manager pattern
pub struct ManagedAsyncServerBuilder<Manager, InternalError, ReplyError>
where
    Manager: Send + Sync + 'static,
    InternalError: Into<ReplyError> + 'static,
    ReplyError: Copy + Sync + Send + 'static,
{
    builder: AsyncServerBuilder,
    manager: Arc<Manager>,
    _phantom: PhantomData<(InternalError, ReplyError)>,
}

impl<Manager, InternalError, ReplyError>
    ManagedAsyncServerBuilder<Manager, InternalError, ReplyError>
where
    Manager: Send + Sync + 'static,
    InternalError: Into<ReplyError> + 'static,
    ReplyError: Copy + Sync + Send + 'static,
{
    /// Creates a new server builder with the given manager, name and version.
    pub fn new(manager: &Arc<Manager>, name: &'static str, version: u16) -> Self {
        Self {
            builder: AsyncServerBuilder::new(name, version),
            manager: manager.clone(),
            _phantom: PhantomData,
        }
    }

    /// Sets a handler for process exit notifications, as a server method.
    pub fn with_process_exit_handler<Fut, F>(mut self, method: F) -> Self
    where
        Fut: Future<Output = ()> + Send + 'static,
        F: Fn(Arc<Manager>, u64) -> Fut + Sync + Send + 'static,
    {
        let manager = self.manager.clone();
        let method = Arc::new(method);

        let handler = move |pid| {
            let instance = manager.clone();
            let method = method.clone();

            async move {
                method(instance, pid).await;
            }
        };

        self.builder = self.builder.with_process_exit_handler(handler);
        self
    }

    /// Adds a message handler without reply for the given message type, as a server method.
    pub fn with_handler_no_reply<QueryParameters, MessageType, Fut, F>(
        mut self,
        message_type: MessageType,
        method: F,
    ) -> Self
    where
        QueryParameters: Copy + Sync + Send + 'static,
        MessageType: Into<u16>,
        Fut: Future<Output = ()> + Sync + Send + 'static,
        F: Fn(Arc<Manager>, QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
    {
        let manager = self.manager.clone();
        let method = Arc::new(method);

        let handler = move |parameters, handles, sender_pid| {
            let instance = manager.clone();
            let method = method.clone();

            async move {
                method(instance, parameters, handles, sender_pid).await;
            }
        };

        self.builder = self.builder.with_handler_no_reply(message_type, handler);
        self
    }

    /// Adds a message handler with reply for the given message type, as a server method.
    pub fn with_handler<QueryParameters, ReplyContent, MessageType, Fut, F>(
        mut self,
        message_type: MessageType,
        method: F,
    ) -> Self
    where
        QueryParameters: Copy + Sync + Send + 'static,
        ReplyContent: Copy + Sync + Send + 'static,
        MessageType: Into<u16>,
        Fut: Future<Output = Result<(ReplyContent, KHandles), InternalError>> + Send + 'static,
        F: Fn(Arc<Manager>, QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
    {
        let manager = self.manager.clone();
        let method = Arc::new(method);

        let handler = move |parameters, handles, sender_pid| {
            let instance = manager.clone();
            let method = method.clone();

            async move {
                let res = method(instance, parameters, handles, sender_pid).await;
                res.map_err(|e| Into::<ReplyError>::into(e))
            }
        };

        self.builder = self.builder.with_handler(message_type, handler);
        self
    }

    /// Builds the server.
    pub fn build(self) -> Result<AsyncServer, kobject::Error> {
        self.builder.build()
    }
}

impl<Manager, InternalError, ReplyError> fmt::Debug
    for ManagedAsyncServerBuilder<Manager, InternalError, ReplyError>
where
    Manager: Send + Sync + 'static,
    InternalError: Into<ReplyError> + 'static,
    ReplyError: Copy + Sync + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.builder.fmt(f)
    }
}

/// Async IPC server.
///
/// Create using `AsyncServerBuilder`.
#[derive(Debug)]
pub struct AsyncServer {
    sender: Option<kobject::PortSender>, // Keep sender alive for named port lookup
    server_port_worker: ServerPortWorker,
    process_listener_worker: Option<ProcessTerminationWorker>,
}

impl AsyncServer {
    fn new(
        name: &str,
        version: u16,
        handlers: HashMap<u16, Box<dyn MessageHandler>>,
        process_exit_handler: Option<Box<dyn ProcessTerminationHandler>>,
    ) -> Result<Self, kobject::Error> {
        let (receiver, sender) = kobject::Port::create(Some(name))?;

        let server_port_worker = ServerPortWorker::new(receiver, version, handlers);

        let process_listener_worker = if let Some(handler) = process_exit_handler {
            let listener = kobject::ProcessListener::create(kobject::ProcessListenerFilter::All)?;
            Some(ProcessTerminationWorker::new(listener, handler))
        } else {
            None
        };

        Ok(Self {
            sender: Some(sender),
            server_port_worker,
            process_listener_worker,
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
        self.server_port_worker.start();

        if let Some(worker) = self.process_listener_worker {
            worker.start();
        }

        r#async::block_on();

        // Port receiver should never complete
        // Process listener (if used) should never complete
        unreachable!();
    }
}

#[derive(Debug)]
struct ProcessTerminationWorker {
    process_listener: kobject::ProcessListener,
    process_exit_handler: Box<dyn ProcessTerminationHandler>,
}

impl ProcessTerminationWorker {
    pub fn new(
        process_listener: kobject::ProcessListener,
        process_exit_handler: Box<dyn ProcessTerminationHandler>,
    ) -> Self {
        Self {
            process_listener,
            process_exit_handler,
        }
    }

    pub fn start(self) {
        r#async::spawn(async move {
            self.run().await;
        });
    }

    async fn run(self) {
        loop {
            r#async::wait(&self.process_listener).await;
            let event = self
                .process_listener
                .receive()
                .expect("failed to receive process event");

            if let kobject::ProcessEventType::Terminated = event.r#type {
                self.process_exit_handler.handle_termination(event.pid);
            }
        }
    }
}

#[derive(Debug)]
struct ServerPortWorker {
    receiver: kobject::PortReceiver,
    version: u16,
    handlers: HashMap<u16, Box<dyn MessageHandler>>,
}

impl ServerPortWorker {
    pub fn new(
        receiver: kobject::PortReceiver,
        version: u16,
        handlers: HashMap<u16, Box<dyn MessageHandler>>,
    ) -> Self {
        Self {
            receiver,
            version,
            handlers,
        }
    }

    pub fn start(self) {
        r#async::spawn(async move {
            self.run().await;
        });
    }

    async fn run(self) {
        loop {
            r#async::wait(&self.receiver).await;

            let message = self
                .receiver
                .receive()
                .expect("failed to receive message from port");

            let header = unsafe { message.data::<QueryHeader>() };
            if header.version != self.version {
                error!("invalid message version: {}", header.version);
                continue;
            }

            let Some(handler) = self.handlers.get(&header.r#type) else {
                error!("no handler for message type: {}", header.r#type);
                continue;
            };

            handler.handle_message(message);
        }
    }
}

trait ProcessTerminationHandler: fmt::Debug + Sync + Send {
    fn handle_termination(&self, pid: u64);
}

struct ProcessTerminationHandlerImpl<F, Fut>
where
    F: Fn(u64) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    handler: Arc<F>,
}

impl<F, Fut> ProcessTerminationHandlerImpl<F, Fut>
where
    F: Fn(u64) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    pub fn new(handler: F) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }
}

impl<F, Fut> ProcessTerminationHandler for ProcessTerminationHandlerImpl<F, Fut>
where
    F: Fn(u64) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn handle_termination(&self, pid: u64) {
        let handler = self.handler.clone();
        r#async::spawn(async move {
            handler(pid).await;
        });
    }
}

impl<F, Fut> fmt::Debug for ProcessTerminationHandlerImpl<F, Fut>
where
    F: Fn(u64) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessTerminationHandlerImpl").finish()
    }
}

trait MessageHandler: fmt::Debug + Sync + Send {
    fn handle_message(&self, message: kobject::Message);
}

struct MessageHandlerWithoutReply<QueryParameters, Fut, F>
where
    QueryParameters: Copy + Sync + Send,
    Fut: Future<Output = ()> + Send + 'static,
    F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
{
    handler: Arc<F>,
    _phantom: PhantomData<QueryParameters>,
}

impl<QueryParameters, Fut, F> MessageHandlerWithoutReply<QueryParameters, Fut, F>
where
    QueryParameters: Copy + Sync + Send,
    Fut: Future<Output = ()> + Send + 'static,
    F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
{
    pub fn new(handler: F) -> Self {
        Self {
            handler: Arc::new(handler),
            _phantom: PhantomData,
        }
    }

    async fn handle_message_async(handler: Arc<F>, mut message: kobject::Message) {
        let handles = message.take_all_handles().into();
        let query = unsafe { message.data::<QueryMessage<QueryParameters>>() };
        handler(query.parameters, handles, query.header.sender_pid).await;
    }
}

impl<QueryParameters, Fut, F> MessageHandler for MessageHandlerWithoutReply<QueryParameters, Fut, F>
where
    QueryParameters: Copy + Sync + Send,
    Fut: Future<Output = ()> + Send + 'static,
    F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
{
    fn handle_message(&self, message: kobject::Message) {
        let handler = self.handler.clone();
        r#async::spawn(async move {
            Self::handle_message_async(handler, message).await;
        });
    }
}

impl<QueryParameters, Fut, F> fmt::Debug for MessageHandlerWithoutReply<QueryParameters, Fut, F>
where
    QueryParameters: Copy + Sync + Send,
    Fut: Future<Output = ()> + Send + 'static,
    F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageHandlerWithoutReply")
    }
}

// Note: Handler 0 = reply port
struct MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError, Fut, F>
where
    QueryParameters: Copy + Sync + Send,
    ReplyContent: Copy + Sync + Send,
    ReplyError: Copy + Sync + Send,
    Fut: Future<Output = Result<(ReplyContent, KHandles), ReplyError>> + Send + 'static,
    F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
{
    handler: Arc<F>,
    _phantom: (
        PhantomData<QueryParameters>,
        PhantomData<ReplyContent>,
        PhantomData<ReplyError>,
    ),
}

impl<QueryParameters, ReplyContent, ReplyError, Fut, F>
    MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError, Fut, F>
where
    QueryParameters: Copy + Sync + Send,
    ReplyContent: Copy + Sync + Send,
    ReplyError: Copy + Sync + Send,
    Fut: Future<Output = Result<(ReplyContent, KHandles), ReplyError>> + Send + 'static,
    F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
{
    pub fn new(handler: F) -> Self {
        Self {
            handler: Arc::new(handler),
            _phantom: (PhantomData, PhantomData, PhantomData),
        }
    }

    async fn handle_message_async(handler: Arc<F>, mut message: kobject::Message) {
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
        let result = handler(query.parameters, handles, query.header.sender_pid).await;

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

impl<QueryParameters, ReplyContent, ReplyError, Fut, F> MessageHandler
    for MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError, Fut, F>
where
    QueryParameters: Copy + Sync + Send,
    ReplyContent: Copy + Sync + Send,
    ReplyError: Copy + Sync + Send,
    Fut: Future<Output = Result<(ReplyContent, KHandles), ReplyError>> + Send + 'static,
    F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
{
    fn handle_message(&self, message: kobject::Message) {
        let handler = self.handler.clone();
        r#async::spawn(async move {
            Self::handle_message_async(handler, message).await;
        });
    }
}

impl<QueryParameters, ReplyContent, ReplyError, Fut, F> fmt::Debug
    for MessageHandlerWithReply<QueryParameters, ReplyContent, ReplyError, Fut, F>
where
    QueryParameters: Copy + Sync + Send,
    ReplyContent: Copy + Sync + Send,
    ReplyError: Copy + Sync + Send,
    Fut: Future<Output = Result<(ReplyContent, KHandles), ReplyError>> + Send + 'static,
    F: Fn(QueryParameters, KHandles, u64) -> Fut + Sync + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageHandlerWithReply")
    }
}
