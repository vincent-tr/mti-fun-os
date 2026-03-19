use alloc::{boxed::Box, fmt, sync::Arc};
use log::warn;
use spin::Mutex;

use crate::{kobject, service};

/// A helper struct that represents an IRQ that can be processed by a service runner.
pub struct RunnableIrq {
    runner: &'static service::Runner,
    runnable: Arc<RunnableObject>,
    component_id: service::ComponentId,
}

unsafe impl Send for RunnableIrq {}
unsafe impl Sync for RunnableIrq {}

impl fmt::Debug for RunnableIrq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RunnableIrq")
            .field("component_id", &self.component_id)
            .field("irq", &self.runnable.irq)
            .finish()
    }
}

impl RunnableIrq {
    /// Creates a new `RunnableIrq` and registers it with the given runner.
    pub fn create(runner: &'static service::Runner) -> Result<Self, kobject::Error> {
        let irq = kobject::Irq::create()?;
        let runnable = Arc::new(RunnableObject::new(irq));

        let component_id = runner.add_component(runnable.clone());

        Ok(Self {
            runner,
            runnable,
            component_id,
        })
    }

    /// Sets the callback to be called when the IRQ is triggered.
    pub fn set_callback(&self, callback: impl Fn(kobject::IrqEvent) + Sync + Send + 'static) {
        self.runnable.set_callback(callback);
    }

    /// Clears the callback, so that when the IRQ is triggered, it will be ignored.
    pub fn clear_callback(&self) {
        self.runnable.clear_callback();
    }

    /// Gets information about the IRQ.
    pub fn info(&self) -> Result<kobject::IrqInfo, kobject::Error> {
        self.runnable
            .irq
            .lock()
            .as_ref()
            .expect("RunnableObject has no IRQ")
            .info()
    }
}

impl Drop for RunnableIrq {
    fn drop(&mut self) {
        self.runner.remove_component(self.component_id);
        self.runnable.clear_callback();
        self.runnable.clear_irq();
    }
}

struct RunnableObject {
    callback: Mutex<Option<Box<dyn Fn(kobject::IrqEvent) + Sync + Send + 'static>>>,
    irq: Mutex<Option<kobject::Irq>>,
}

impl fmt::Debug for RunnableObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RunnableObject")
            .field("irq", &self.irq)
            .field(
                "has_callback",
                &self.callback.lock().as_ref().map(|_| true).unwrap_or(false),
            )
            .finish()
    }
}

impl RunnableObject {
    pub fn new(irq: kobject::Irq) -> Self {
        Self {
            callback: Mutex::new(None),
            irq: Mutex::new(Some(irq)),
        }
    }

    pub fn clear_irq(&self) {
        *self.irq.lock() = None;
    }

    pub fn set_callback(&self, callback: impl Fn(kobject::IrqEvent) + Sync + Send + 'static) {
        *self.callback.lock() = Some(Box::new(callback));
    }

    pub fn clear_callback(&self) {
        *self.callback.lock() = None;
    }
}

impl service::RunnableComponent for RunnableObject {
    fn process(&self) {
        let irq_lock = self.irq.lock();
        let callback_lock = self.callback.lock();

        let irq = irq_lock.as_ref().expect("RunnableObject has no IRQ");
        let callback = &*callback_lock;

        let event = match irq.receive() {
            Ok(event) => event,
            Err(e) => {
                warn!("Failed to receive IRQ event: {:?}", e);
                return;
            }
        };

        if let Some(callback) = callback {
            callback(event);
        } else {
            warn!("IRQ {:?} triggered but no callback is set", irq);
        }
    }
}

impl kobject::KWaitable for RunnableObject {
    unsafe fn waitable_handle(&self) -> &libsyscalls::Handle {
        let irq_lock = self.irq.lock();

        let irq = irq_lock.as_ref().expect("RunnableObject has no IRQ");
        // The IRQ won't be dropped during the wait phase, so we can safely keep a reference to the handle.
        let handle_ptr = unsafe { irq.waitable_handle() as *const libsyscalls::Handle };
        unsafe { &*handle_ptr }
    }

    fn wait(&self) -> Result<(), kobject::Error> {
        let irq_lock = self.irq.lock();

        let irq = irq_lock.as_ref().expect("RunnableObject has no IRQ");
        irq.wait()
    }
}
