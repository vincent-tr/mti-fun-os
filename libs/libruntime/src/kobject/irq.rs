use libsyscalls::irq;

use super::*;

/// Irq object, used to wait for an IRQ
///
/// Note: since IRQ is a complex object, it partially implements KObject
#[derive(Debug)]
pub struct Irq {
    irq: Handle,
    reader: PortReceiver,
}

impl KObject for Irq {
    unsafe fn handle(&self) -> &Handle {
        &self.irq
    }

    fn into_handle(self) -> Handle {
        self.irq
    }

    unsafe fn from_handle_unchecked(_handle: Handle) -> Self {
        panic!("Irq cannot be created from handle directly");
    }

    fn from_handle(_handle: Handle) -> Result<Self, Error> {
        Err(Error::NotSupported)
    }
}

impl KWaitable for Irq {
    unsafe fn waitable_handle(&self) -> &Handle {
        unsafe { self.reader.waitable_handle() }
    }

    fn wait(&self) -> Result<(), Error> {
        self.reader.wait()
    }
}

impl Irq {
    /// Create a new object which listen to IRQ events.
    pub fn create() -> Result<Self, Error> {
        let (reader, sender) = Port::create(None)?;
        let irq = irq::create(unsafe { sender.handle() })?;

        Ok(Self { irq, reader })
    }

    /// Get information about the IRQ
    pub fn info(&self) -> Result<IrqInfo, Error> {
        let info = irq::info(&self.irq)?;

        Ok(info)
    }

    /// Receive an IRQ event
    ///
    /// Note: the call does not block, it returns ObjectNotReady if no message is waiting
    pub fn receive(&self) -> Result<IrqEvent, Error> {
        let msg = self.reader.receive()?;

        Ok(unsafe { msg.data::<IrqEvent>().clone() })
    }

    /// Receive an IRQ event, blocking until one is available.
    pub fn blocking_receive(&self) -> Result<IrqEvent, Error> {
        let msg = self.reader.blocking_receive()?;

        Ok(unsafe { msg.data::<IrqEvent>().clone() })
    }
}
