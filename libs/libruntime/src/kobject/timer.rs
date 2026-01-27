use libsyscalls::timer;

use super::*;

/// Timer object
#[derive(Debug)]
pub struct Timer {
    id: u64,
    timer: Handle,
    reader: PortReceiver,
}

impl KObject for Timer {
    unsafe fn handle(&self) -> &Handle {
        &self.timer
    }
}

impl KWaitable for Timer {
    unsafe fn waitable_handle(&self) -> &Handle {
        self.reader.waitable_handle()
    }

    fn wait(&self) -> Result<(), Error> {
        self.reader.wait()
    }
}

impl Timer {
    /// Create a new timer object
    pub fn create(id: u64) -> Result<Self, Error> {
        let (reader, sender) = Port::create(None)?;
        let timer = timer::create(unsafe { sender.handle() }, id)?;

        Ok(Self { id, timer, reader })
    }

    /// Arm the timer to trigger at deadline
    pub fn arm(&self, deadline: u64) -> Result<(), Error> {
        timer::arm(&self.timer, deadline)?;

        Ok(())
    }

    /// Cancel the timer
    pub fn cancel(&self) -> Result<(), Error> {
        timer::cancel(&self.timer)?;

        // Flush any pending timer events
        self.reader.receive()?;

        Ok(())
    }

    /// Get the timer ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Receive a timer event
    ///
    /// Note: the call does not block, it returns ObjectNotReady if no message is waiting
    pub fn receive(&self) -> Result<TimerEvent, Error> {
        let msg = self.reader.receive()?;

        Ok(unsafe { msg.data::<TimerEvent>().clone() })
    }

    /// Block until a timer event is received
    pub fn blocking_receive(&self) -> Result<TimerEvent, Error> {
        let msg = self.reader.blocking_receive()?;

        Ok(unsafe { msg.data::<TimerEvent>().clone() })
    }

    /// Get the current monotonic time
    pub fn now() -> Result<u64, Error> {
        timer::now()
    }
}
