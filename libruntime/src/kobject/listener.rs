use alloc::vec::Vec;
use libsyscalls::listener;

use super::*;

/// Thread listener
///
/// Note: since thread listener is a complex object, it does not implement KObject directly
#[derive(Debug)]
pub struct ThreadListener {
    tids: Option<Vec<u64>>,
    _listener: Handle,
    reader: PortReceiver,
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
    pub fn create(tids: Option<&[u64]>) -> Result<Self, Error> {
        let (reader, sender) = Port::create(None)?;
        let listener = listener::create_thread(unsafe { sender.handle() }, tids)?;
        let tids = tids.map(|list| Vec::from(list));

        Ok(Self {
            tids,
            _listener: listener,
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

    /// Get the thread ids that are registered to this listener.
    ///
    /// Note: None means no filter: receive all events
    pub fn tids(&self) -> Option<&[u64]> {
        self.tids.as_ref().map(|list| -> &[u64] { &list })
    }
}

/// Process listener
///
/// Note: since thread listener is a complex object, it does not implement KObject directly
#[derive(Debug)]
pub struct ProcessListener {
    pids: Option<Vec<u64>>,
    _listener: Handle,
    reader: PortReceiver,
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
    pub fn create(pids: Option<&[u64]>) -> Result<Self, Error> {
        let (reader, sender) = Port::create(None)?;
        let listener = listener::create_process(unsafe { sender.handle() }, pids)?;
        let pids = pids.map(|list| Vec::from(list));

        Ok(Self {
            pids,
            _listener: listener,
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

    /// Get the process ids that are registered to this listener.
    ///
    /// Note: None means no filter: receive all events
    pub fn pids(&self) -> Option<&[u64]> {
        self.pids.as_ref().map(|list| -> &[u64] { &list })
    }
}
