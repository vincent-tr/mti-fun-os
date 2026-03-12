use core::fmt;

use alloc::{sync::Arc, vec::Vec};
use hashbrown::HashMap;

use crate::{kobject, sync::Mutex};

pub trait RunnableComponent: kobject::KWaitable {
    fn process(&self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(usize);

/// IPC runner module.
#[derive(Debug)]
pub struct Runner(Mutex<RunnerData>);

impl Runner {
    /// Creates a new IPC runner.
    pub fn new() -> Self {
        Self(Mutex::new(RunnerData::new()))
    }

    /// Adds a component to the runner.
    pub fn add_component(&self, component: Arc<dyn RunnableComponent>) -> ComponentId {
        self.0.lock().add_component(component)
    }

    pub fn remove_component(&self, id: ComponentId) {
        self.0.lock().remove_component(id);
    }

    /// Runs the IPC runner, processing events for all components.
    pub fn run(&self) -> ! {
        loop {
            let waiter = self.0.lock().waiter();
            waiter.run();
        }
    }
}

#[derive(Debug)]
struct RunnerData {
    components: HashMap<ComponentId, Arc<dyn RunnableComponent>>,
    cached_waiter: Option<Arc<CachedWaiter>>,
    id_gen: usize,
}

impl RunnerData {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            cached_waiter: None,
            id_gen: 0,
        }
    }

    pub fn add_component(&mut self, component: Arc<dyn RunnableComponent>) -> ComponentId {
        self.id_gen += 1;
        let id = ComponentId(self.id_gen);
        self.components.insert(id, component);
        self.cached_waiter = None; // Invalidate cached waiter
        id
    }

    pub fn remove_component(&mut self, id: ComponentId) {
        self.components.remove(&id);
        self.cached_waiter = None; // Invalidate cached waiter
    }

    pub fn waiter(&mut self) -> Arc<CachedWaiter> {
        if self.cached_waiter.is_none() {
            let components = self.components.values().cloned().collect::<Vec<_>>();
            self.cached_waiter = Some(Arc::new(CachedWaiter::new(components)));
        }

        self.cached_waiter
            .as_ref()
            .expect("cached_waiter is none")
            .clone()
    }
}

/// A cached waiter that holds references to the components and their waitable pointers.
///
/// This allows the waiter to be reused without needing to reconstruct the list of waitable pointers on every run, improving performance when the set of components is stable.
struct CachedWaiter {
    list: Vec<Arc<dyn RunnableComponent>>,
    waiter: Mutex<kobject::Waiter<'static>>,
}

impl fmt::Debug for CachedWaiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedWaiter")
            .field("list_len", &self.list.len())
            .finish()
    }
}

impl CachedWaiter {
    pub fn new(components: Vec<Arc<dyn RunnableComponent>>) -> Self {
        let mut waiter = kobject::Waiter::new(&[]);
        for component in components.iter() {
            // Safety: we ensure that the components live as long as the waiter, and we never modify the list after creating the waiter
            let comp_ref = unsafe { &*Arc::as_ptr(component) };
            waiter.add(comp_ref);
        }

        Self {
            list: components,
            waiter: Mutex::new(waiter),
        }
    }

    pub fn run(&self) {
        let mut waiter = self.waiter.lock();

        waiter.wait().expect("wait failed");

        for (index, component) in self.list.iter().enumerate() {
            if waiter.is_ready(index) {
                component.process();
            }
        }
    }
}
