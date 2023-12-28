use core::mem;

use alloc::sync::Arc;
use hashbrown::HashMap;
use spin::RwLock;
use syscalls::HandleType;

use super::{
    error::{check_arg_opt, invalid_argument},
    id_gen::IdGen,
    process::Process,
    thread::Thread,
    Error, MemoryObject,
};

/// Handle: Pointer to kernel object, usable from userland
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Handle(u64);

impl From<u64> for Handle {
    fn from(value: u64) -> Self {
        Handle(value)
    }
}

impl From<usize> for Handle {
    fn from(value: usize) -> Self {
        Handle(value as u64)
    }
}

impl Handle {
    /// Construct a new invalid handle
    pub const fn invalid() -> Self {
        Handle(0)
    }

    /// Indicate is the handle is valid
    pub const fn valid(&self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, Clone)]
enum HandleImpl {
    MemoryObjectHandle(Arc<MemoryObject>),
    ProcessHandle(Arc<Process>),
    ThreadHandle(Arc<Thread>),
}

impl HandleImpl {
    pub fn r#type(&self) -> HandleType {
        match self {
            HandleImpl::MemoryObjectHandle(_) => HandleType::MemoryObject,
            HandleImpl::ProcessHandle(_) => HandleType::Process,
            HandleImpl::ThreadHandle(_) => HandleType::Thread,
        }
    }
}

/// Handles management in a process
#[derive(Debug)]
pub struct Handles {
    id_gen: IdGen,
    handles: RwLock<HashMap<Handle, HandleImpl>>,
}

impl Handles {
    pub fn new() -> Self {
        Self {
            id_gen: IdGen::new(),
            handles: RwLock::new(HashMap::new()),
        }
    }

    /// Open the given memory object in the process
    pub fn open_memory_object(&self, memory_object: Arc<MemoryObject>) -> Handle {
        self.open(HandleImpl::MemoryObjectHandle(memory_object))
    }

    /// Open the given process in the process
    pub fn open_process(&self, process: Arc<Process>) -> Handle {
        self.open(HandleImpl::ProcessHandle(process))
    }

    /// Open the given thread in the process
    pub fn open_thread(&self, thread: Arc<Thread>) -> Handle {
        self.open(HandleImpl::ThreadHandle(thread))
    }

    fn open(&self, handle_impl: HandleImpl) -> Handle {
        let handle = Handle(self.id_gen.generate());

        let mut handles = self.handles.write();
        handles.insert_unique_unchecked(handle, handle_impl);

        handle
    }

    /// Retrieve the type of the handle
    pub fn r#type(&self, handle: Handle) -> Result<HandleType, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        Ok(handle_impl.r#type())
    }

    /// Retrieve the memory object from the handle
    pub fn get_memory_object(&self, handle: Handle) -> Result<Arc<MemoryObject>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let HandleImpl::MemoryObjectHandle(memory_object) = handle_impl {
            Ok(memory_object.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the process from the handle
    pub fn get_process(&self, handle: Handle) -> Result<Arc<Process>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let HandleImpl::ProcessHandle(process) = handle_impl {
            Ok(process.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the thread from the handle
    pub fn get_thread(&self, handle: Handle) -> Result<Arc<Thread>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let HandleImpl::ThreadHandle(thread) = handle_impl {
            Ok(thread.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Close the handle
    pub fn close(&self, handle: Handle) -> Result<(), Error> {
        let mut handles = self.handles.write();

        let handle_impl = check_arg_opt(handles.remove(&handle))?;

        // Let's be explicit
        mem::drop(handle_impl);

        Ok(())
    }

    /// Duplicate a handle
    pub fn duplicate(&self, handle: Handle) -> Result<Handle, Error> {
        let new_handle_impl = {
            let handles = self.handles.read();

            let handle_impl = check_arg_opt(handles.get(&handle))?;

            handle_impl.clone()
        };

        Ok(self.open(new_handle_impl))
    }
}
