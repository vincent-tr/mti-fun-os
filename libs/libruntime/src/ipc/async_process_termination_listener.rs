use crate::{r#async, kobject};
use alloc::{boxed::Box, sync::Arc};
use core::fmt;

/// Listener for process termination events.
pub struct AsyncProcessTerminationListener {
    listener: kobject::ProcessListener,
    handler: Box<dyn Handler>,
}

impl fmt::Debug for AsyncProcessTerminationListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncProcessTerminationListener").finish()
    }
}

impl AsyncProcessTerminationListener {
    /// Creates a new process termination listener with the given manager method as handler.
    pub fn from_handler_method<Manager, Fut, F>(
        manager: &Arc<Manager>,
        method: F,
    ) -> Result<Self, kobject::Error>
    where
        Manager: Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        F: Fn(Arc<Manager>, u64) -> Fut + Sync + Send + 'static,
    {
        let manager = manager.clone();
        let method = Arc::new(method);

        let handler = move |pid| {
            let instance = manager.clone();
            let method = method.clone();

            async move {
                method(instance, pid).await;
            }
        };

        Self::from_handler(handler)
    }

    /// Creates a new process termination listener with the given handler.
    pub fn from_handler<Fut, F>(handler: F) -> Result<Self, kobject::Error>
    where
        Fut: Future<Output = ()> + Send + 'static,
        F: Fn(u64) -> Fut + Sync + Send + 'static,
    {
        Ok(Self {
            listener: kobject::ProcessListener::create(kobject::ProcessListenerFilter::All)?,
            handler: Box::new(HandlerImpl::new(handler)),
        })
    }

    /// Starts the listener in a new asynchronous task.
    pub fn start(self) {
        r#async::spawn(async move {
            self.run().await;
        });
    }

    async fn run(self) {
        loop {
            r#async::wait(&self.listener).await;
            let event = self
                .listener
                .receive()
                .expect("failed to receive process event");

            if let kobject::ProcessEventType::Terminated = event.r#type {
                self.handler.run(event.pid);
            }
        }
    }
}

trait Handler: Sync + Send {
    fn run(&self, pid: u64);
}

struct HandlerImpl<F, Fut>
where
    F: Fn(u64) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    handler: Arc<F>,
}

impl<F, Fut> HandlerImpl<F, Fut>
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

impl<F, Fut> Handler for HandlerImpl<F, Fut>
where
    F: Fn(u64) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn run(&self, pid: u64) {
        let handler = self.handler.clone();
        r#async::spawn(async move {
            handler(pid).await;
        });
    }
}
