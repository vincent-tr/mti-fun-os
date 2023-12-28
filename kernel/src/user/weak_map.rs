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
        self.clean_map();

        let mut map = self.map.write();
        assert!(
            map.insert(id, Arc::downgrade(&value)).is_none(),
            "unepxected map overwrite"
        );
    }

    pub fn has(&self, id: &Key) -> bool {
        self.clean_map();

        let map = self.map.read();

        map.contains_key(id)
    }

    /// Find a an item by its id
    pub fn find(&self, id: &Key) -> Option<Arc<Value>> {
        self.clean_map();

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
        self.clean_map();

        let map = self.map.read();
        map.keys().map(|key| key.clone()).collect()
    }

    /// Get the number of items in the map
    pub fn len(&self) -> usize {
        self.clean_map();

        let map = self.map.read();
        map.len()
    }

    fn clean_map(&self) {
        let map = self.map.upgradeable_read();

        let mut delete_list = Vec::new();

        for (id, weak) in map.iter() {
            if weak.strong_count() == 0 {
                delete_list.push(id.clone());
            }
        }

        if delete_list.len() > 0 {
            let mut map = map.upgrade();
            for id in delete_list {
                map.remove(&id);
            }
        }
    }
}
