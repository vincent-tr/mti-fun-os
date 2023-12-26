use hashbrown::HashMap;

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use core::hash::Hash;
use spin::RwLock;

#[derive(Debug)]
pub struct WeakMap<Key: Copy + Eq + Hash, Value> {
    map: RwLock<HashMap<Key, Weak<Value>>>,
}

impl<Key: Copy + Eq + Hash, Value> WeakMap<Key, Value> {
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

    /// Find a an item by its id
    pub fn find(&self, id: Key) -> Option<Arc<Value>> {
        self.clean_map();

        let map = self.map.read();
        if let Some(weak) = map.get(&id) {
            return weak.upgrade();
        } else {
            None
        }
    }

    fn clean_map(&self) {
        let map = self.map.upgradeable_read();

        let mut delete_list = Vec::new();

        for (&id, weak) in map.iter() {
            if weak.strong_count() == 0 {
                delete_list.push(id);
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
