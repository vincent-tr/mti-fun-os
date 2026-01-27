use core::sync::atomic::{AtomicU64, Ordering};

use alloc::sync::Arc;
use log::debug;
use syscalls::TimerEvent;

use crate::user::{
    ipc::{MessageBuilder, PortSender},
    Error,
};

/// Represent a disabled deadline, by setting a value that is never reachable (too far in the future)
const DISABLED_DEADLINE: u64 = u64::MAX;

/// Represent a timer object
#[derive(Debug)]
pub struct Timer {
    port: Arc<PortSender>,
    id: u64,
    deadline: AtomicU64,
}

impl Timer {
    /// Create a new timer
    pub fn new(port: Arc<PortSender>, id: u64) -> Result<Arc<Self>, Error> {
        Ok(Arc::new(Self {
            port,
            id,
            deadline: AtomicU64::new(DISABLED_DEADLINE),
        }))
    }

    pub fn arm(&self, deadline: u64) {
        self.deadline.store(deadline, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.deadline.store(DISABLED_DEADLINE, Ordering::SeqCst);
    }

    fn tick(&self, now: u64) {
        let deadline = self.deadline.load(Ordering::SeqCst);
        if now >= deadline {
            // disable the timer
            self.deadline.store(DISABLED_DEADLINE, Ordering::SeqCst);

            self.send_event(now);
        }
    }

    fn send_event(&self, now: u64) {
        let mut builder = MessageBuilder::new();
        let event = builder.data_mut::<TimerEvent>();
        event.id = self.id;
        event.now = now;

        match self.port.kernel_send(builder.message()) {
            Ok(()) => {}
            Err(err) => {
                debug!(
                    "Failed to send TimerEvent message to port {}: {:?}",
                    self.port.id(),
                    err
                );
            }
        }
    }
}

// Make it external so that we can keep it private to the module
pub fn timer_tick(timer: &Timer, now: u64) {
    timer.tick(now);
}
