use core::sync::atomic::AtomicU64;

use alloc::{sync::Arc, vec::Vec};
use hashbrown::HashMap;

use crate::sync::RwLock;

/// Represents a client handle to a server object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(u64);

impl Handle {
    /// Returns an invalid handle.
    pub const fn invalid() -> Self {
        Handle(0)
    }

    /// Checks if the handle is valid.
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl From<u64> for Handle {
    fn from(value: u64) -> Self {
        Handle(value)
    }
}

/// A generator for unique handles.
///
/// It is designed to be part of the state of the server, so that it can be persisted and survive server restart.
/// It is also thread-safe, allowing concurrent handle generation from multiple threads if needed.
#[derive(Debug, Default)]
#[repr(C)]
pub struct HandleGenerator {
    last_handle: AtomicU64,
}

impl HandleGenerator {
    // Note: State structures are not directly created, they are accessed through StateView.
    // Its default value will be zero-initialized, which means the first generated handle will be 1 (since 0 is reserved as invalid).

    /// Generates a new unique handle.
    pub fn generate(&self) -> Handle {
        let handle_value = self
            .last_handle
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst)
            + 1;
        Handle(handle_value)
    }
}

/// A table to track what client process opened handles to server objects.
#[derive(Debug)]
pub struct HandleTable<'a, T> {
    table: RwLock<HashMap<u64, HashMap<Handle, Arc<T>>>>,
    generator: &'a HandleGenerator,
}

impl<'a, T> HandleTable<'a, T> {
    /// Creates a new handle table.
    pub fn new(generator: &'a HandleGenerator) -> Self {
        HandleTable {
            table: RwLock::new(HashMap::new()),
            generator,
        }
    }

    /// Registers that a process has terminated, removing all its opened handles.
    pub fn process_terminated(&self, process_id: u64) -> Vec<Arc<T>> {
        let mut table = self.table.write();

        if let Some(handles) = table.remove(&process_id) {
            handles.into_values().collect()
        } else {
            Vec::new()
        }
    }

    /// Opens a new handle for the given process and object.
    pub fn open(&self, process_id: u64, object: Arc<T>) -> Handle {
        let mut table = self.table.write();

        let process_handles = table.entry(process_id).or_insert_with(HashMap::new);

        let handle = self.generator.generate();

        process_handles.insert(handle, object);

        handle
    }

    /// Closes a handle for the given process, returning the associated object if it existed.
    pub fn close(&self, process_id: u64, handle: Handle) -> Option<Arc<T>> {
        let mut table = self.table.write();

        if let Some(process_handles) = table.get_mut(&process_id) {
            process_handles.remove(&handle)
        } else {
            None
        }
    }

    /// Reads the object associated with a handle for the given process.
    pub fn read(&self, process_id: u64, handle: Handle) -> Option<Arc<T>> {
        let table = self.table.read();

        if let Some(process_handles) = table.get(&process_id) {
            process_handles.get(&handle).cloned()
        } else {
            None
        }
    }
}
