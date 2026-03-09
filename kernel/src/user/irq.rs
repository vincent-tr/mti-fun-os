use core::ptr;

use alloc::sync::Arc;
use log::{debug, warn};
use spin::RwLock;
use syscalls::IrqEvent;

use super::Error;

use crate::{
    interrupts,
    user::ipc::{MessageBuilder, PortSender},
};

// Redefine const as usize for easier usage in array indexing and arithmetic
const IRQ_START: usize = interrupts::EXTERNAL_IRQ_START as usize;
const IRQ_END: usize = interrupts::EXTERNAL_IRQ_END as usize;

/// IRQ (Interrupt Request) handling for user space.
#[derive(Debug)]
pub struct Irq {
    vector: usize,
    port: Arc<PortSender>,
}

impl Drop for Irq {
    fn drop(&mut self) {
        let mut table = IRQ_TABLE.write();

        table.remove(self.vector);
    }
}

impl Irq {
    /// Create a new IRQ object associated with the given port.
    ///
    /// The IRQ number will be automatically assigned from the available range.
    pub fn new(port: Arc<PortSender>) -> Result<Arc<Self>, Error> {
        let mut table = IRQ_TABLE.write();

        let vector = table.next_free().ok_or(Error::OutOfMemory)?;
        let irq = Arc::new(Self { vector, port });
        table.add(vector, &irq);

        Ok(irq)
    }

    /// Get the vector associated with this IRQ object.
    pub fn vector(&self) -> usize {
        self.vector
    }

    /// Send an IrqEvent message to the associated port to notify that the IRQ has been triggered.
    fn fire(&self) {
        let mut builder = MessageBuilder::new();

        let event = builder.data_mut::<IrqEvent>();
        event.vector = self.vector as u64;

        match self.port.kernel_send(builder.message()) {
            Ok(()) => {}
            Err(err) => {
                debug!(
                    "Failed to send IrqEvent message to port {}: {:?}",
                    self.port.id(),
                    err
                );
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
struct IrqPtr(*const Irq);

impl IrqPtr {
    pub const fn null() -> Self {
        Self(ptr::null())
    }

    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub unsafe fn as_ref(&self) -> &'static Irq {
        assert!(!self.is_null(), "Attempted to dereference a null IrqPtr");
        unsafe { &*self.0 }
    }
}

impl From<&Arc<Irq>> for IrqPtr {
    fn from(value: &Arc<Irq>) -> Self {
        Self(value.as_ref() as *const Irq)
    }
}

unsafe impl Send for IrqPtr {}
unsafe impl Sync for IrqPtr {}

/// Interrupt object table for easier dispatching and usage check
#[derive(Debug)]
struct Table([IrqPtr; IRQ_END - IRQ_START + 1]);

impl Table {
    /// Create a new empty table.
    pub const fn new() -> Self {
        Self([IrqPtr::null(); IRQ_END - IRQ_START + 1])
    }

    /// Get the next free IRQ number that can be registered, or None if the table is full.
    pub fn next_free(&self) -> Option<usize> {
        for (index, entry) in self.0.iter().enumerate() {
            if entry.is_null() {
                return Some(IRQ_START + index);
            }
        }

        None
    }

    /// Add an IRQ object to the table, associating it with the specified IRQ number.
    pub fn add(&mut self, vector: usize, irq: &Arc<Irq>) {
        assert!(vector >= IRQ_START && vector <= IRQ_END);

        let index = vector - (IRQ_START as usize);

        assert!(
            self.0[index].is_null(),
            "IRQ {} is already registered",
            vector
        );
        self.0[index] = IrqPtr::from(irq);
    }

    /// Remove the IRQ object associated with the specified IRQ number from the table.
    pub fn remove(&mut self, vector: usize) {
        assert!(vector >= IRQ_START && vector <= IRQ_END);

        let index = (vector - IRQ_START) as usize;

        assert!(!self.0[index].is_null(), "IRQ {} is not registered", vector);
        self.0[index] = IrqPtr::null();
    }

    /// Get the IRQ object associated with the specified IRQ number, or None if no object is registered for that IRQ.
    pub fn get(&self, vector: usize) -> Option<&Irq> {
        assert!(vector >= IRQ_START && vector <= IRQ_END);

        let entry = self.0[(vector - IRQ_START) as usize];
        if entry.is_null() {
            None
        } else {
            Some(unsafe { entry.as_ref() })
        }
    }
}

lazy_static::lazy_static! {
    static ref IRQ_TABLE: RwLock<Table> = RwLock::new(Table::new());
}

/// Called by the ISR management code when a device IRQ is triggered.
pub fn handle_irq(vector: u8) {
    debug!("Handling IRQ {}", vector);

    let table = IRQ_TABLE.read();

    if let Some(irq) = table.get(vector as usize) {
        irq.fire()
    } else {
        warn!(
            "Unhandled IRQ {} triggered with no registered handler",
            vector
        );
    }
}
