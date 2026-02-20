use core::num::NonZeroUsize;

use alloc::string::String;
use libruntime::{
    sync::Mutex,
    vfs::{NodeType, Permissions},
};
use lru::LruCache;

use crate::vnode::VNode;

/// Maximum number of entries in the attributes cache.
const ATTRIBUTES_CACHE_SIZE: usize = 1024;

/// Cache for vnode attributes
///
/// Note: whole metadata is not cached, only type and permissions, as other attributes (like size) can change frequently and are not needed for most operations.
#[derive(Debug, Clone)]
pub struct NodeAttributes {
    r#type: NodeType,
    permissions: Permissions,
}

/// Cache for vnode attributes, keyed by vnode.
#[derive(Debug)]
pub struct NodeAttributesCache {
    cache: Mutex<LruCache<VNode, NodeAttributes>>,
}

impl NodeAttributesCache {
    /// Get the global instance of the node attributes cache.
    pub fn get() -> &'static Self {
        lazy_static::lazy_static! {
            static ref INSTANCE: NodeAttributesCache = NodeAttributesCache::new();
        }
        &INSTANCE
    }

    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(ATTRIBUTES_CACHE_SIZE)
                    .expect("Tried to create a cache with a max of 0 entries"),
            )),
        }
    }

    /// Get the cached attributes for a vnode, if present.
    pub fn fetch(&self, vnode: &VNode) -> Option<NodeAttributes> {
        let mut cache = self.cache.lock();
        cache.get(vnode).cloned()
    }

    /// Update the cached attributes for a vnode.
    pub fn update(&self, vnode: VNode, r#type: NodeType, permissions: Permissions) {
        let attributes = NodeAttributes {
            r#type,
            permissions,
        };
        let mut cache = self.cache.lock();
        cache.put(vnode, attributes);
    }
}

/// Maximum number of entries in the lookup cache.
const LOOKUP_CACHE_SIZE: usize = 1024;

/// Cache key for directory lookups, consisting of the parent vnode and the name of the entry being looked up.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LookupKey {
    node: VNode,
    name: String,
}

/// Cache for vnode attributes, keyed by vnode.
#[derive(Debug)]
pub struct LookupCache {
    cache: Mutex<LruCache<LookupKey, VNode>>,
}

impl LookupCache {
    /// Get the global instance of the lookup cache.
    pub fn get() -> &'static Self {
        lazy_static::lazy_static! {
            static ref INSTANCE: LookupCache = LookupCache::new();
        }
        &INSTANCE
    }

    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(LOOKUP_CACHE_SIZE)
                    .expect("Tried to create a cache with a max of 0 entries"),
            )),
        }
    }

    /// Get the cached vnode for a lookup key, if present.
    pub fn fetch(&self, dir: VNode, name: &str) -> Option<VNode> {
        let key = LookupKey {
            node: dir,
            name: String::from(name),
        };
        let mut cache = self.cache.lock();
        cache.get(&key).cloned()
    }

    /// Update the cached vnode for a lookup key.
    pub fn update(&self, dir: VNode, name: String, vnode: VNode) {
        let key = LookupKey {
            node: dir,
            name: name,
        };
        let mut cache = self.cache.lock();
        cache.put(key, vnode);
    }
}
