use core::sync::atomic::AtomicU64;

use alloc::vec::Vec;
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

/*
/// A table to track what client process opened handles to server objects.
#[derive(Debug)]
pub struct HandleTable<T> {
    table: RwLock<HashMap<u64, HashMap<Handle, T>>>,
    next_handle: AtomicU64,
}

impl<T> HandleTable<T> {
    /// Creates a new handle table.
    pub fn new() -> Self {
        HandleTable {
            table: RwLock::new(HashMap::new()),
            next_handle: AtomicU64::new(1),
        }
    }

    /// Registers that a process has terminated, removing all its opened handles.
    pub fn process_terminated(&self, process_id: u64) -> Vec<T> {
        let mut table = self.table.write();

        if let Some(handles) = table.remove(&process_id) {
            handles.into_values().collect()
        } else {
            Vec::new()
        }
    }

    /// Opens a new handle for the given process and object.
    pub fn open(&self, process_id: u64, object: T) -> Handle {
        let mut table = self.table.write();

        let process_handles = table.entry(process_id).or_insert_with(HashMap::new);

        let handle_value = self
            .next_handle
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        let handle = Handle(handle_value);

        process_handles.insert(handle, object);

        handle
    }

    /// Closes a handle for the given process, returning the associated object if it existed.
    pub fn close(&self, process_id: u64, handle: Handle) -> Option<T> {
        let mut table = self.table.write();

        if let Some(process_handles) = table.get_mut(&process_id) {
            process_handles.remove(&handle)
        } else {
            None
        }
    }

    /// Reads the object associated with a handle for the given process.
    pub fn read(&self, process_id: u64, handle: Handle) -> Option<&T> {
        let table = self.table.read();

        if let Some(process_handles) = table.get(&process_id) {
            process_handles.get(&handle)
        } else {
            None
        }
    }
}
*/
