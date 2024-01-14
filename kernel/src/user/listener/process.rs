use alloc::{boxed::Box, sync::Arc};
use core::{fmt::Debug, marker::PhantomPinned, pin::Pin};
use hashbrown::HashSet;
use lazy_static::lazy_static;
use log::debug;
use syscalls::{ProcessEvent, ProcessEventType};

use crate::user::{ipc::PortSender, process::Process};

use super::{message_builder::MessageBuilder, ListenerList};

lazy_static! {
    static ref LISTENERS: ListenerList<ProcessListener> = ListenerList::new();
}

pub fn notify_process(process: &Process, r#type: ProcessEventType) {
    LISTENERS.notify(|listener| listener.notify(process, r#type));
}

/// Represent a process listener
#[derive(Debug)]
pub struct ProcessListener {
    filter: Box<dyn Filter>,
    port: Arc<PortSender>,
    _marker: PhantomPinned,
}

unsafe impl Sync for ProcessListener {}
unsafe impl Send for ProcessListener {}

impl ProcessListener {
    pub fn new(port: Arc<PortSender>, pids: Option<&[u64]>) -> Pin<Arc<Self>> {
        let filter = if let Some(list) = pids {
            PidsFilter::new(list)
        } else {
            AllFilter::new()
        };

        let listener = Arc::pin(Self {
            port,
            filter,
            _marker: PhantomPinned,
        });

        // Note: need not move since we keep tracks of pointers
        LISTENERS.add(&listener);

        listener
    }

    fn notify(&self, process: &Process, r#type: ProcessEventType) {
        if !self.filter.filter(process) {
            return;
        }

        let mut builder = MessageBuilder::new();

        let event = builder.data_mut::<ProcessEvent>();
        event.pid = process.id();
        event.r#type = r#type;

        match self.port.kernel_send(builder.message()) {
            Ok(()) => {}
            Err(err) => {
                debug!(
                    "Failed to send ProcessEvent message to port {}: {:?}",
                    self.port.id(),
                    err
                );
            }
        }
    }
}

impl Drop for ProcessListener {
    fn drop(&mut self) {
        LISTENERS.remove(self);
    }
}

trait Filter: Debug {
    fn filter(&self, process: &Process) -> bool;
}

#[derive(Debug)]
struct AllFilter {}

impl AllFilter {
    pub fn new() -> Box<dyn Filter> {
        Box::new(Self {})
    }
}

impl Filter for AllFilter {
    fn filter(&self, _process: &Process) -> bool {
        true
    }
}

#[derive(Debug)]
struct PidsFilter {
    allowed: HashSet<u64>,
}

impl PidsFilter {
    pub fn new(ids: &[u64]) -> Box<dyn Filter> {
        let allowed = HashSet::from_iter(ids.iter().copied());

        Box::new(Self { allowed })
    }
}

impl Filter for PidsFilter {
    fn filter(&self, process: &Process) -> bool {
        self.allowed.contains(&process.id())
    }
}
