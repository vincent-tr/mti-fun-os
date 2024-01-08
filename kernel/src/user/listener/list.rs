use core::{hash::Hash, ptr::NonNull};

use hashbrown::HashSet;
use spin::RwLock;

#[derive(Debug, Copy, Clone)]
struct ListenerPtr<TListener>(NonNull<TListener>);

impl<TListener> ListenerPtr<TListener> {
    pub fn new(value: &TListener) -> Self {
        Self(NonNull::from(value))
    }
}

// Not sure why derive macro does not work here?
impl<TListener> Hash for ListenerPtr<TListener> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

// Not sure why derive macro does not work here?
impl<TListener> PartialEq for ListenerPtr<TListener> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// Not sure why derive macro does not work here?
impl<TListener> Eq for ListenerPtr<TListener> {}

unsafe impl<TListener> Send for ListenerPtr<TListener> {}
unsafe impl<TListener> Sync for ListenerPtr<TListener> {}

#[derive(Debug)]
pub struct ListenerList<TListener>(RwLock<HashSet<ListenerPtr<TListener>>>);

impl<TListener> ListenerList<TListener> {
    pub fn new() -> Self {
        Self(RwLock::new(HashSet::new()))
    }

    pub fn add(&self, listener: &TListener) {
        let mut set = self.0.write();

        set.insert(ListenerPtr::new(listener));
    }

    pub fn remove(&self, listener: &TListener) {
        let mut set = self.0.write();

        set.remove(&ListenerPtr::new(listener));
    }

    pub fn notify<F: Fn(&TListener)>(&self, notifier: F) {
        let set = self.0.read();
        for listener in set.iter() {
            notifier(unsafe { listener.0.as_ref() });
        }
    }
}
