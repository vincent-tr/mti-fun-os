use alloc::{boxed::Box, sync::Arc};
use core::{fmt::Debug, marker::PhantomPinned, pin::Pin};
use hashbrown::HashSet;
use lazy_static::lazy_static;
use log::debug;
use syscalls::{ThreadEvent, ThreadEventType};

use crate::user::{
    ipc::{MessageBuilder, PortSender},
    thread::Thread,
};

use super::ListenerList;

lazy_static! {
    static ref LISTENERS: ListenerList<ThreadListener> = ListenerList::new();
}

pub fn notify_thread(thread: &Thread, r#type: ThreadEventType) {
    LISTENERS.notify(|listener| listener.notify(thread, r#type));
}

/// Represent a thread listener
#[derive(Debug)]
pub struct ThreadListener {
    filter: Box<dyn Filter>,
    port: Arc<PortSender>,
    _marker: PhantomPinned,
}

unsafe impl Sync for ThreadListener {}
unsafe impl Send for ThreadListener {}

impl ThreadListener {
    pub fn new(port: Arc<PortSender>, ids: Option<&[u64]>, is_pids: bool) -> Pin<Arc<Self>> {
        let filter = if let Some(list) = ids {
            if is_pids {
                PidsFilter::new(list)
            } else {
                TidsFilter::new(list)
            }
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

    fn notify(&self, thread: &Thread, r#type: ThreadEventType) {
        if !self.filter.filter(thread) {
            return;
        }

        let mut builder = MessageBuilder::new();

        let event = builder.data_mut::<ThreadEvent>();
        event.tid = thread.id();
        event.r#type = r#type;

        match self.port.kernel_send(builder.message()) {
            Ok(()) => {}
            Err(err) => {
                debug!(
                    "Failed to send ThreadEvent message to port {}: {:?}",
                    self.port.id(),
                    err
                );
            }
        }
    }
}

impl Drop for ThreadListener {
    fn drop(&mut self) {
        LISTENERS.remove(self);
    }
}

trait Filter: Debug {
    fn filter(&self, thread: &Thread) -> bool;
}

#[derive(Debug)]
struct AllFilter {}

impl AllFilter {
    pub fn new() -> Box<dyn Filter> {
        Box::new(Self {})
    }
}

impl Filter for AllFilter {
    fn filter(&self, _thread: &Thread) -> bool {
        true
    }
}

#[derive(Debug)]
struct TidsFilter {
    allowed: HashSet<u64>,
}

impl TidsFilter {
    pub fn new(ids: &[u64]) -> Box<dyn Filter> {
        let allowed = HashSet::from_iter(ids.iter().copied());

        Box::new(Self { allowed })
    }
}

impl Filter for TidsFilter {
    fn filter(&self, thread: &Thread) -> bool {
        self.allowed.contains(&thread.id())
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
    fn filter(&self, thread: &Thread) -> bool {
        self.allowed.contains(&thread.process().id())
    }
}
