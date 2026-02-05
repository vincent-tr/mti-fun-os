use hashbrown::HashMap;

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{fmt::Debug, hash::Hash};

#[derive(Debug)]
pub struct WeakMap<Key: Eq + Hash + Clone, Value> {
    map: HashMap<Key, Weak<Value>>,
}

impl<Key: Eq + Hash + Clone + Debug, Value> WeakMap<Key, Value> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Insert item
    pub fn insert(&mut self, id: Key, value: &Arc<Value>) {
        assert!(
            self.map
                .insert(id.clone(), Arc::downgrade(&value))
                .is_none(),
            "unexpected map overwrite: {id:?}"
        );
    }

    /// Remove item
    pub fn remove(&mut self, id: Key) {
        assert!(
            self.map.remove(&id).is_some(),
            "unexpected map remove with no value: {id:?}"
        );
    }

    pub fn has(&self, id: &Key) -> bool {
        self.map.contains_key(id)
    }

    /// Find a an item by its id
    pub fn find(&self, id: &Key) -> Option<Arc<Value>> {
        if let Some(weak) = self.map.get(id) {
            return weak.upgrade();
        } else {
            None
        }
    }

    /// List map keys
    ///
    /// Note:
    /// The data is copied to avoid to keep the map locked
    pub fn keys(&self) -> Vec<Key> {
        self.map.keys().map(|key| key.clone()).collect()
    }

    /// Get the number of items in the map
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Lookup (slowly) for a value in the map.
    ///
    /// Returns the first occurence that matchs the predicate, if any
    pub fn lookup<Predicate: Fn(&Arc<Value>) -> bool>(
        &self,
        predicate: Predicate,
    ) -> Option<Arc<Value>> {
        for value in self.map.values() {
            if let Some(value) = value.upgrade() {
                if predicate(&value) {
                    return Some(value);
                }
            }
        }

        None
    }
}
