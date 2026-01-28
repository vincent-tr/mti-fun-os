use alloc::vec::Vec;
use libsyscalls::listener;

use super::*;

/// Indicate the filter type on thread listener
#[derive(Debug)]
pub enum ThreadListenerFilter<'a> {
    All,
    Tids(&'a [u64]),
    Pids(&'a [u64]),
}

impl<'a> ThreadListenerFilter<'a> {
    fn syscall_arg(&self) -> (Option<&'a [u64]>, bool) {
        match self {
            ThreadListenerFilter::All => (None, false),
            ThreadListenerFilter::Tids(list) => (Some(list), false),
            ThreadListenerFilter::Pids(list) => (Some(list), true),
        }
    }
}

#[derive(Debug)]
enum ThreadListenerFilterOwner {
    All,
    Tids(Vec<u64>),
    Pids(Vec<u64>),
}

impl From<ThreadListenerFilter<'_>> for ThreadListenerFilterOwner {
    fn from(value: ThreadListenerFilter) -> Self {
        match value {
            ThreadListenerFilter::All => ThreadListenerFilterOwner::All,
            ThreadListenerFilter::Tids(list) => ThreadListenerFilterOwner::Tids(Vec::from(list)),
            ThreadListenerFilter::Pids(list) => ThreadListenerFilterOwner::Pids(Vec::from(list)),
        }
    }
}

impl ThreadListenerFilterOwner {
    fn as_ref(&self) -> ThreadListenerFilter<'_> {
        match self {
            ThreadListenerFilterOwner::All => ThreadListenerFilter::All,
            ThreadListenerFilterOwner::Tids(list) => ThreadListenerFilter::Tids(list.as_slice()),
            ThreadListenerFilterOwner::Pids(list) => ThreadListenerFilter::Pids(list.as_slice()),
        }
    }
}

/// Thread listener
///
/// Note: since thread listener is a complex object, it does not implement KObject directly
#[derive(Debug)]
pub struct ThreadListener {
    filter: ThreadListenerFilterOwner,
    listener: Handle,
    reader: PortReceiver,
}

impl KObject for ThreadListener {
    unsafe fn handle(&self) -> &Handle {
        &self.listener
    }

    fn into_handle(self) -> Handle {
        self.listener
    }
}

impl KWaitable for ThreadListener {
    unsafe fn waitable_handle(&self) -> &Handle {
        self.reader.waitable_handle()
    }

    fn wait(&self) -> Result<(), Error> {
        self.reader.wait()
    }
}

impl ThreadListener {
    /// Create a new object which listen to thread event.
    pub fn create(filter: ThreadListenerFilter) -> Result<Self, Error> {
        let (reader, sender) = Port::create(None)?;
        let (ids, is_pids) = filter.syscall_arg();
        let listener = listener::create_thread(unsafe { sender.handle() }, ids, is_pids)?;

        Ok(Self {
            filter: ThreadListenerFilterOwner::from(filter),
            listener,
            reader,
        })
    }

    /// Receive a thread event
    ///
    /// Note: the call does not block, it returns ObjectNotReady if no message is waiting
    pub fn receive(&self) -> Result<ThreadEvent, Error> {
        let msg = self.reader.receive()?;

        Ok(unsafe { msg.data::<ThreadEvent>().clone() })
    }

    /// Block until a thread event is received
    pub fn blocking_receive(&self) -> Result<ThreadEvent, Error> {
        let msg = self.reader.blocking_receive()?;

        Ok(unsafe { msg.data::<ThreadEvent>().clone() })
    }

    /// Get the filter that is setup on this listener.
    pub fn filter(&self) -> ThreadListenerFilter {
        self.filter.as_ref()
    }
}

/// Indicate the filter type on process listener
#[derive(Debug)]
pub enum ProcessListenerFilter<'a> {
    All,
    Pids(&'a [u64]),
}

impl<'a> ProcessListenerFilter<'a> {
    fn syscall_arg(&self) -> Option<&'a [u64]> {
        match self {
            ProcessListenerFilter::All => None,
            ProcessListenerFilter::Pids(list) => Some(list),
        }
    }
}

#[derive(Debug)]
enum ProcessListenerFilterOwner {
    All,
    Pids(Vec<u64>),
}

impl From<ProcessListenerFilter<'_>> for ProcessListenerFilterOwner {
    fn from(value: ProcessListenerFilter) -> Self {
        match value {
            ProcessListenerFilter::All => ProcessListenerFilterOwner::All,
            ProcessListenerFilter::Pids(list) => ProcessListenerFilterOwner::Pids(Vec::from(list)),
        }
    }
}

impl ProcessListenerFilterOwner {
    fn as_ref(&self) -> ProcessListenerFilter<'_> {
        match self {
            ProcessListenerFilterOwner::All => ProcessListenerFilter::All,
            ProcessListenerFilterOwner::Pids(list) => ProcessListenerFilter::Pids(list.as_slice()),
        }
    }
}

/// Process listener
///
/// Note: since thread listener is a complex object, it does not implement KObject directly
#[derive(Debug)]
pub struct ProcessListener {
    filter: ProcessListenerFilterOwner,
    listener: Handle,
    reader: PortReceiver,
}

impl KObject for ProcessListener {
    unsafe fn handle(&self) -> &Handle {
        &self.listener
    }

    fn into_handle(self) -> Handle {
        self.listener
    }
}

impl KWaitable for ProcessListener {
    unsafe fn waitable_handle(&self) -> &Handle {
        self.reader.waitable_handle()
    }

    fn wait(&self) -> Result<(), Error> {
        self.reader.wait()
    }
}

impl ProcessListener {
    /// Create a new object which listen to process event.
    pub fn create(filter: ProcessListenerFilter) -> Result<Self, Error> {
        let (reader, sender) = Port::create(None)?;
        let pids = filter.syscall_arg();
        let listener = listener::create_process(unsafe { sender.handle() }, pids)?;

        Ok(Self {
            filter: ProcessListenerFilterOwner::from(filter),
            listener,
            reader,
        })
    }

    /// Receive a process event
    ///
    /// Note: the call does not block, it returns ObjectNotReady if no message is waiting
    pub fn receive(&self) -> Result<ProcessEvent, Error> {
        let msg = self.reader.receive()?;

        Ok(unsafe { msg.data::<ProcessEvent>().clone() })
    }

    /// Block until a process event is received
    pub fn blocking_receive(&self) -> Result<ProcessEvent, Error> {
        let msg = self.reader.blocking_receive()?;

        Ok(unsafe { msg.data::<ProcessEvent>().clone() })
    }

    /// Get the filter that is setup on this listener.
    pub fn filter(&self) -> ProcessListenerFilter {
        self.filter.as_ref()
    }
}
