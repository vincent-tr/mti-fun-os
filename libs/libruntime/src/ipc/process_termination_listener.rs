use super::RunnableComponent;
use crate::kobject;
use alloc::{boxed::Box, sync::Arc};
use core::fmt;

/// Listener for process termination events.
pub struct ProcessTerminationListener {
    listener: kobject::ProcessListener,
    handler: Box<dyn Fn(u64) + 'static>,
}

impl kobject::KWaitable for ProcessTerminationListener {
    unsafe fn waitable_handle(&self) -> &libsyscalls::Handle {
        unsafe { self.listener.waitable_handle() }
    }

    fn wait(&self) -> Result<(), kobject::Error> {
        self.listener.wait()
    }
}

impl RunnableComponent for ProcessTerminationListener {
    fn process(&self) {
        let event = self
            .listener
            .receive()
            .expect("failed to receive process event");

        if let kobject::ProcessEventType::Terminated = event.r#type {
            (self.handler)(event.pid);
        }
    }
}

impl fmt::Debug for ProcessTerminationListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessTerminationListener").finish()
    }
}

impl ProcessTerminationListener {
    /// Creates a new process termination listener with the given manager method as handler.
    pub fn from_handler_method<Manager, F>(
        manager: &Arc<Manager>,
        method: F,
    ) -> Result<Self, kobject::Error>
    where
        Manager: 'static,
        F: Fn(&Manager, u64) + 'static,
    {
        let manager = manager.clone();
        let handler = move |pid| {
            let instance = manager.clone();
            method(&instance, pid);
        };

        Self::from_handler(handler)
    }

    /// Creates a new process termination listener with the given handler.
    pub fn from_handler<F>(handler: F) -> Result<Self, kobject::Error>
    where
        F: Fn(u64) + 'static,
    {
        Ok(Self {
            listener: kobject::ProcessListener::create(kobject::ProcessListenerFilter::All)?,
            handler: Box::new(handler),
        })
    }
}
