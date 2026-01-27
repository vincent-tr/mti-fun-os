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
}
