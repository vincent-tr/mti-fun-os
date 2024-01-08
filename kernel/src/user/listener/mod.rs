mod filters;
mod list;
mod message_builder;

use self::{
    filters::{AllFilter, IdFilter, ListFilter},
    list::ListenerList,
    message_builder::MessageBuilder,
};
use alloc::{boxed::Box, sync::Arc};
use core::fmt::Debug;
use lazy_static::lazy_static;
use log::debug;
use syscalls::{ProcessEvent, ThreadEvent};
pub use syscalls::{ProcessEventType, ThreadEventType};

use super::ipc::PortSender;

lazy_static! {
    static ref PROCESS_LISTENERS: ListenerList<ProcessListener> = ListenerList::new();
    static ref THREAD_LISTENERS: ListenerList<ThreadListener> = ListenerList::new();
}

pub fn notify_process(pid: u64, r#type: ProcessEventType) {
    PROCESS_LISTENERS.notify(|listener| listener.notify(pid, r#type));
}

pub fn notify_thread(tid: u64, r#type: ThreadEventType) {
    THREAD_LISTENERS.notify(|listener| listener.notify(tid, r#type));
}

/// Represent a process listener
#[derive(Debug)]
pub struct ProcessListener {
    filter: Box<dyn IdFilter>,
    port: Arc<PortSender>,
}

unsafe impl Sync for ProcessListener {}
unsafe impl Send for ProcessListener {}

impl ProcessListener {
    pub fn new(port: Arc<PortSender>, pids: Option<&[u64]>) -> Arc<Self> {
        let filter = if let Some(list) = pids {
            ListFilter::new(list)
        } else {
            AllFilter::new()
        };

        Arc::new(Self { port, filter })
    }

    fn notify(&self, pid: u64, r#type: ProcessEventType) {
        if !self.filter.filter(pid) {
            return;
        }

        let mut builder = MessageBuilder::new();

        let event = builder.data_mut::<ProcessEvent>();
        event.pid = pid;
        event.r#type = r#type;

        match self.port.kernel_send(builder.message()) {
            Ok(()) => {}
            Err(err) => {
                debug!(
                    "Failed to send ProcessEvent message to port {:?}: {:?}",
                    self.port, err
                );
            }
        }
    }
}

impl Drop for ProcessListener {
    fn drop(&mut self) {
        PROCESS_LISTENERS.remove(self);
    }
}

/// Represent a thread listener
#[derive(Debug)]
pub struct ThreadListener {
    filter: Box<dyn IdFilter>,
    port: Arc<PortSender>,
}

unsafe impl Sync for ThreadListener {}
unsafe impl Send for ThreadListener {}

impl ThreadListener {
    pub fn new(port: Arc<PortSender>, tids: Option<&[u64]>) -> Arc<Self> {
        let filter = if let Some(list) = tids {
            ListFilter::new(list)
        } else {
            AllFilter::new()
        };

        Arc::new(Self { port, filter })
    }

    fn notify(&self, tid: u64, r#type: ThreadEventType) {
        if !self.filter.filter(tid) {
            return;
        }

        let mut builder = MessageBuilder::new();

        let event = builder.data_mut::<ThreadEvent>();
        event.tid = tid;
        event.r#type = r#type;

        match self.port.kernel_send(builder.message()) {
            Ok(()) => {}
            Err(err) => {
                debug!(
                    "Failed to send ThreadEvent message to port {:?}: {:?}",
                    self.port, err
                );
            }
        }
    }
}

impl Drop for ThreadListener {
    fn drop(&mut self) {
        THREAD_LISTENERS.remove(self);
    }
}
