use hashbrown::HashMap;

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use core::hash::Hash;
use spin::RwLock;

#[derive(Debug)]
pub struct WeakMap<Key: Eq + Hash + Clone, Value> {
    map: RwLock<HashMap<Key, Weak<Value>>>,
}

impl<Key: Eq + Hash + Clone, Value> WeakMap<Key, Value> {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Insert item
    pub fn insert(&self, id: Key, value: &Arc<Value>) {
        let mut map = self.map.write();
        assert!(
            map.insert(id, Arc::downgrade(&value)).is_none(),
            "unexpected map overwrite"
        );
    }

    /// Remove item
    pub fn remove(&self, id: Key) {
        let mut map = self.map.write();
        assert!(
            map.remove(&id).is_some(),
            "unexpected map remove with no value"
        );
    }

    pub fn has(&self, id: &Key) -> bool {
        let map = self.map.read();

        map.contains_key(id)
    }

    /// Find a an item by its id
    pub fn find(&self, id: &Key) -> Option<Arc<Value>> {
        let map = self.map.read();
        if let Some(weak) = map.get(id) {
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
        let map = self.map.read();
        map.keys().map(|key| key.clone()).collect()
    }

    /// Get the number of items in the map
    pub fn len(&self) -> usize {
        let map = self.map.read();
        map.len()
    }

    /// Lookup (slowly) for a value in the map.
    ///
    /// Returns the first occurence that matchs the predicate, if any
    pub fn lookup<Predicate: Fn(&Arc<Value>) -> bool>(
        &self,
        predicate: Predicate,
    ) -> Option<Arc<Value>> {
        let map = self.map.read();
        for value in map.values() {
            if let Some(value) = value.upgrade() {
                if predicate(&value) {
                    return Some(value);
                }
            }
        }

        None
    }
}
