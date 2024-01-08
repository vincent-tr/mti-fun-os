use core::mem;

use alloc::sync::Arc;
use hashbrown::HashMap;
use spin::RwLock;
use syscalls::HandleType;

use super::{
    error::{check_arg_opt, invalid_argument},
    id_gen::IdGen,
    ipc::{Port, PortReceiver, PortSender},
    listener::{ProcessListener, ThreadListener},
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

    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone)]
pub enum KernelHandle {
    MemoryObjectHandle(Arc<MemoryObject>),
    ProcessHandle(Arc<Process>),
    ThreadHandle(Arc<Thread>),
    PortReceiverHandle(Arc<PortReceiver>),
    PortSenderHandle(Arc<PortSender>),
    ProcessListenerHandle(Arc<ProcessListener>),
    ThreadListenerHandle(Arc<ThreadListener>),
}

impl KernelHandle {
    pub fn r#type(&self) -> HandleType {
        match self {
            KernelHandle::MemoryObjectHandle(_) => HandleType::MemoryObject,
            KernelHandle::ProcessHandle(_) => HandleType::Process,
            KernelHandle::ThreadHandle(_) => HandleType::Thread,
            KernelHandle::PortReceiverHandle(_) => HandleType::PortReceiver,
            KernelHandle::PortSenderHandle(_) => HandleType::PortSender,
            KernelHandle::ProcessListenerHandle(_) => HandleType::ProcessListener,
            KernelHandle::ThreadListenerHandle(_) => HandleType::ThreadListener,
        }
    }

    /// Check if the 2 handles points to the same object
    pub fn is_obj_eq(&self, other: &KernelHandle) -> bool {
        match self {
            KernelHandle::MemoryObjectHandle(self_obj) => {
                if let KernelHandle::MemoryObjectHandle(other_obj) = other {
                    Arc::ptr_eq(self_obj, other_obj)
                } else {
                    false
                }
            }
            KernelHandle::ProcessHandle(self_obj) => {
                if let KernelHandle::ProcessHandle(other_obj) = other {
                    Arc::ptr_eq(self_obj, other_obj)
                } else {
                    false
                }
            }
            KernelHandle::ThreadHandle(self_obj) => {
                if let KernelHandle::ThreadHandle(other_obj) = other {
                    Arc::ptr_eq(self_obj, other_obj)
                } else {
                    false
                }
            }
            KernelHandle::PortReceiverHandle(self_obj) => {
                if let KernelHandle::PortReceiverHandle(other_obj) = other {
                    Arc::ptr_eq(self_obj, other_obj)
                } else {
                    false
                }
            }
            KernelHandle::PortSenderHandle(self_obj) => {
                if let KernelHandle::PortSenderHandle(other_obj) = other {
                    Arc::ptr_eq(self_obj, other_obj)
                } else {
                    false
                }
            }
            KernelHandle::ProcessListenerHandle(self_obj) => {
                if let KernelHandle::ProcessListenerHandle(other_obj) = other {
                    Arc::ptr_eq(self_obj, other_obj)
                } else {
                    false
                }
            }
            KernelHandle::ThreadListenerHandle(self_obj) => {
                if let KernelHandle::ThreadListenerHandle(other_obj) = other {
                    Arc::ptr_eq(self_obj, other_obj)
                } else {
                    false
                }
            }
        }
    }
}

/// Handles management in a process
#[derive(Debug)]
pub struct Handles {
    id_gen: IdGen,
    handles: RwLock<HashMap<Handle, KernelHandle>>,
}

impl Handles {
    pub fn new() -> Self {
        Self {
            id_gen: IdGen::new(),
            handles: RwLock::new(HashMap::new()),
        }
    }

    /// Get the number of opened handles
    pub fn len(&self) -> usize {
        let handles = self.handles.read();

        handles.len()
    }

    /// Open the given memory object in the process
    pub fn open_memory_object(&self, memory_object: Arc<MemoryObject>) -> Handle {
        self.open(KernelHandle::MemoryObjectHandle(memory_object))
    }

    /// Open the given process in the process
    pub fn open_process(&self, process: Arc<Process>) -> Handle {
        self.open(KernelHandle::ProcessHandle(process))
    }

    /// Open the given thread in the process
    pub fn open_thread(&self, thread: Arc<Thread>) -> Handle {
        self.open(KernelHandle::ThreadHandle(thread))
    }

    /// Open the given port receiver in the process
    pub fn open_port_receiver(&self, port: Arc<PortReceiver>) -> Handle {
        self.open(KernelHandle::PortReceiverHandle(port))
    }

    /// Open the given port sender in the process
    pub fn open_port_sender(&self, port: Arc<PortSender>) -> Handle {
        self.open(KernelHandle::PortSenderHandle(port))
    }

    /// Open the given process listener in the process
    pub fn open_process_listener(&self, listener: Arc<ProcessListener>) -> Handle {
        self.open(KernelHandle::ProcessListenerHandle(listener))
    }

    /// Open the given thread listener in the process
    pub fn open_thread_listener(&self, listener: Arc<ThreadListener>) -> Handle {
        self.open(KernelHandle::ThreadListenerHandle(listener))
    }

    /// Open raw kernel handle
    pub fn open(&self, handle_impl: KernelHandle) -> Handle {
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

    pub fn is_obj_eq(&self, handle1: Handle, handle2: Handle) -> Result<bool, Error> {
        let handles = self.handles.read();

        let handle1_impl = check_arg_opt(handles.get(&handle1))?;
        let handle2_impl = check_arg_opt(handles.get(&handle2))?;

        Ok(handle1_impl.is_obj_eq(handle2_impl))
    }

    pub fn get(&self, handle: Handle) -> Result<KernelHandle, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        Ok(handle_impl.clone())
    }

    /// Retrieve the memory object from the handle
    pub fn get_memory_object(&self, handle: Handle) -> Result<Arc<MemoryObject>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let KernelHandle::MemoryObjectHandle(memory_object) = handle_impl {
            Ok(memory_object.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the process from the handle
    pub fn get_process(&self, handle: Handle) -> Result<Arc<Process>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let KernelHandle::ProcessHandle(process) = handle_impl {
            Ok(process.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the thread from the handle
    pub fn get_thread(&self, handle: Handle) -> Result<Arc<Thread>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let KernelHandle::ThreadHandle(thread) = handle_impl {
            Ok(thread.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the port receiver from the handle
    pub fn get_port_receiver(&self, handle: Handle) -> Result<Arc<PortReceiver>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let KernelHandle::PortReceiverHandle(port_receiver) = handle_impl {
            Ok(port_receiver.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the port sender from the handle
    pub fn get_port_sender(&self, handle: Handle) -> Result<Arc<PortSender>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let KernelHandle::PortSenderHandle(port_sender) = handle_impl {
            Ok(port_sender.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the port sender or receiver, then get the inner port
    pub fn get_port(&self, handle: Handle) -> Result<Arc<Port>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let KernelHandle::PortSenderHandle(port_sender) = handle_impl {
            Ok(port_sender.port().clone())
        } else if let KernelHandle::PortReceiverHandle(port_receiver) = handle_impl {
            Ok(port_receiver.port().clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the process listener from the handle
    pub fn get_process_listener(&self, handle: Handle) -> Result<Arc<ProcessListener>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let KernelHandle::ProcessListenerHandle(process_listener) = handle_impl {
            Ok(process_listener.clone())
        } else {
            Err(invalid_argument())
        }
    }

    /// Retrieve the thread listener from the handle
    pub fn get_thread_listener(&self, handle: Handle) -> Result<Arc<ThreadListener>, Error> {
        let handles = self.handles.read();

        let handle_impl = check_arg_opt(handles.get(&handle))?;

        if let KernelHandle::ThreadListenerHandle(thread_listener) = handle_impl {
            Ok(thread_listener.clone())
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

    /// Close all the handles in the container
    pub fn clear(&self) {
        let mut handles = self.handles.write();

        // Note: all 'handle_impl' will be dropped
        handles.clear();
    }
}
