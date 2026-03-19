use core::ptr::NonNull;

use alloc::{boxed::Box, fmt, sync::Arc};
use log::warn;
use spin::Mutex;

use crate::{kobject, service};

/// A helper struct that represents an IRQ that can be processed by a service runner.
pub struct RunnableIrq {
    runner: &'static service::Runner,
    runnable: Arc<RunnableObject>,
    component_id: service::ComponentId,
    irq: kobject::Irq,
    callback: Mutex<Option<Box<dyn Fn() + Sync + Send + 'static>>>,
}

unsafe impl Send for RunnableIrq {}
unsafe impl Sync for RunnableIrq {}

impl fmt::Debug for RunnableIrq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RunnableIrq")
            .field("component_id", &self.component_id)
            .field("irq", &self.irq)
            .finish()
    }
}

impl RunnableIrq {
    /// Creates a new `RunnableIrq` and registers it with the given runner.
    pub fn create(runner: &'static service::Runner) -> Result<Self, kobject::Error> {
        let irq = kobject::Irq::create()?;

        let runnable = Arc::new(RunnableObject::new());

        let component_id = runner.add_component(runnable.clone());

        let runnable_irq = Self {
            runner,
            runnable,
            component_id,
            irq,
            callback: Mutex::new(None),
        };

        runnable_irq.runnable.set_owner(&runnable_irq);

        Ok(runnable_irq)
    }

    /// Sets the callback to be called when the IRQ is triggered.
    pub fn set_callback(&self, callback: impl Fn() + Sync + Send + 'static) {
        *self.callback.lock() = Some(Box::new(callback));
    }

    /// Clears the callback, so that when the IRQ is triggered, it will be ignored.
    pub fn clear_callback(&self) {
        *self.callback.lock() = None;
    }

    /// Gets information about the IRQ.
    pub fn info(&self) -> Result<kobject::IrqInfo, kobject::Error> {
        self.irq.info()
    }

    fn process(&self) {
        if let Some(callback) = &*self.callback.lock() {
            callback();
        } else {
            warn!("IRQ {:?} triggered but no callback is set", self.irq);
        }
    }
}

impl Drop for RunnableIrq {
    fn drop(&mut self) {
        self.runner.remove_component(self.component_id);
        self.runnable.clear_owner();
    }
}

#[derive(Debug)]
struct RunnableObject {
    owner: Mutex<Option<NonNull<RunnableIrq>>>,
}

unsafe impl Send for RunnableObject {}
unsafe impl Sync for RunnableObject {}

impl RunnableObject {
    pub fn new() -> Self {
        Self {
            owner: Mutex::new(None),
        }
    }

    pub fn set_owner(&self, owner: &RunnableIrq) {
        *self.owner.lock() = Some(NonNull::from(owner));
    }

    pub fn clear_owner(&self) {
        *self.owner.lock() = None;
    }
}

impl service::RunnableComponent for RunnableObject {
    fn process(&self) {
        if let Some(owner) = *self.owner.lock() {
            unsafe { owner.as_ref().process() }
        }
    }
}

impl kobject::KWaitable for RunnableObject {
    unsafe fn waitable_handle(&self) -> &libsyscalls::Handle {
        let owner_ptr = self.owner.lock().expect("RunnableObject has no owner");
        let owner = unsafe { owner_ptr.as_ref() };
        unsafe { owner.irq.waitable_handle() }
    }

    fn wait(&self) -> Result<(), kobject::Error> {
        let owner_ptr = self.owner.lock().expect("RunnableObject has no owner");
        let owner = unsafe { owner_ptr.as_ref() };
        owner.irq.wait()
    }
}
