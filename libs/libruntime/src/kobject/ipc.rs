use core::{
    fmt::Debug,
    mem::{self, size_of},
};

use alloc::vec::Vec;
use bit_field::BitArray;
use libsyscalls::{HandleType, ipc};

type SysMessage = libsyscalls::Message;

use super::*;

pub struct Port {
    _priv: (),
}

impl Port {
    /// Create a new port
    ///
    /// Note: the name only describe the sender part. If it is dropped, the name will be freed, even if the receiver still exists
    pub fn create(name: Option<&str>) -> Result<(PortReceiver, PortSender), Error> {
        let (receiver, sender) = ipc::create(name)?;

        Ok((
            unsafe { PortReceiver::from_handle_unchecked(receiver) },
            unsafe { PortSender::from_handle_unchecked(sender) },
        ))
    }

    /// Open an existing port by name
    pub fn open_by_name(name: &str) -> Result<PortSender, Error> {
        let sender = ipc::open(ipc::NameOrId::Name(name))?;

        Ok(unsafe { PortSender::from_handle_unchecked(sender) })
    }

    /// Open an existing port by id
    pub fn open_by_id(id: u64) -> Result<PortSender, Error> {
        let sender = ipc::open(ipc::NameOrId::Id(id))?;

        Ok(unsafe { PortSender::from_handle_unchecked(sender) })
    }
}

/// Port sender
#[derive(Debug, Clone)]
pub struct PortSender {
    handle: Handle,
}

impl KObject for PortSender {
    unsafe fn handle(&self) -> &Handle {
        &self.handle
    }

    fn into_handle(self) -> Handle {
        self.handle
    }

    unsafe fn from_handle_unchecked(handle: Handle) -> Self {
        Self { handle }
    }

    fn from_handle(handle: Handle) -> Result<Self, Error> {
        if !handle.valid() {
            return Err(Error::InvalidArgument);
        }
        if handle.r#type() != HandleType::PortSender {
            return Err(Error::InvalidArgument);
        }

        Ok(unsafe { Self::from_handle_unchecked(handle) })
    }
}

impl PortSender {
    /// Send a message in the port
    pub fn send(&self, message: &mut Message) -> Result<(), Error> {
        let msg = message.to_send_syscall();

        ipc::send(&self.handle, &msg)?;

        // Need to cleanup the handles, they have been moved into kernel port message
        message.after_send_success();

        Ok(())
    }
}

/// Port receiver
///
/// Note: cloning the receiver duplicates the underlying handle only
/// This means that for each clone, messages may be received in any of the clones, NOT all of them
#[derive(Debug, Clone)]
pub struct PortReceiver {
    handle: Handle,
}

impl KObject for PortReceiver {
    unsafe fn handle(&self) -> &Handle {
        &self.handle
    }

    fn into_handle(self) -> Handle {
        self.handle
    }

    unsafe fn from_handle_unchecked(handle: Handle) -> Self {
        Self { handle }
    }

    fn from_handle(handle: Handle) -> Result<Self, Error> {
        if !handle.valid() {
            return Err(Error::InvalidArgument);
        }
        if handle.r#type() != HandleType::PortReceiver {
            return Err(Error::InvalidArgument);
        }

        Ok(unsafe { Self::from_handle_unchecked(handle) })
    }
}

impl KWaitable for PortReceiver {
    unsafe fn waitable_handle(&self) -> &Handle {
        &self.handle
    }

    fn wait(&self) -> Result<(), Error> {
        let ports = &[unsafe { self.handle.as_syscall_value() }];
        let ready = &mut [0u8];

        ipc::wait(ports, ready)?;

        assert!(ready.get_bit(0));

        Ok(())
    }
}

impl PortReceiver {
    /// Safety: caller must ensure the handle is a valid port receiver handle
    pub unsafe fn from_handle_unchecked(handle: Handle) -> Self {
        Self { handle }
    }

    pub fn from_handle(handle: Handle) -> Result<Self, Error> {
        if !handle.valid() {
            return Err(Error::InvalidArgument);
        }
        if handle.r#type() != HandleType::PortReceiver {
            return Err(Error::InvalidArgument);
        }

        Ok(unsafe { Self::from_handle_unchecked(handle) })
    }

    /// Receive a message from the port
    ///
    /// Note: the call does not block, it returns ObjectNotReady if no message is waiting
    pub fn receive(&self) -> Result<Message, Error> {
        let msg = ipc::receive(&self.handle)?;

        Ok(unsafe { Message::from_receive_syscall(msg) })
    }

    /// Block until a message is received
    pub fn blocking_receive(&self) -> Result<Message, Error> {
        loop {
            self.wait()?;

            match self.receive() {
                Err(Error::ObjectNotReady) => {
                    // retry
                }
                other => {
                    return other;
                }
            }
        }
    }
}

/// Trait to be implemented by all waitable objects
pub trait KWaitable: Debug {
    /// Get the internal waitable handle of the object
    unsafe fn waitable_handle(&self) -> &Handle;

    /// Wait until the object is ready
    fn wait(&self) -> Result<(), Error>;
}

/// Waiter for ports
#[derive(Debug)]
pub struct Waiter<'a> {
    /// Keep this list for user queries
    waitables: Vec<&'a dyn KWaitable>,
    /// Keep this list for efficiency
    handles: Vec<usize>,
    ready: Vec<u8>,
}

impl<'a> Waiter<'a> {
    /// Construct a new port waiter from a list of ports
    pub fn new(waitables: &[&'a dyn KWaitable]) -> Self {
        let mut waiter = Self {
            waitables: Vec::from(waitables),
            handles: Vec::new(),
            ready: Vec::new(),
        };

        waiter.handles.reserve(waiter.len());
        for waitable in waiter.waitables.iter() {
            waiter
                .handles
                .push(unsafe { waitable.waitable_handle().as_syscall_value() });
        }

        waiter.ready_resize();

        waiter
    }

    /// Get the number of waitables
    pub fn len(&self) -> usize {
        self.waitables.len()
    }

    /// Add a waitable at the end of the list
    ///
    /// Note: This reset readyness
    pub fn add(&mut self, waitable: &'a dyn KWaitable) {
        self.waitables.push(waitable);
        self.handles
            .push(unsafe { waitable.waitable_handle().as_syscall_value() });
        self.ready_resize();
    }

    /// Remove the waitable at the specified index
    ///
    /// Note: This reset readyness
    pub fn remove(&mut self, index: usize) {
        self.waitables.remove(index);
        self.handles.remove(index);
        self.ready_resize();
    }

    /// Wait for any waitable to be ready.
    ///
    /// After this call returns, the ready list is updated
    pub fn wait(&mut self) -> Result<(), Error> {
        ipc::wait(&self.handles, &mut self.ready)?;

        Ok(())
    }

    /// Set all reeady flags to fals
    pub fn reset(&mut self) {
        self.ready.fill(0);
    }

    /// Iterate over waitable, readyness tuples
    pub fn iter() {}

    /// Get the waitable at index
    pub fn waitable(&self, index: usize) -> &'a dyn KWaitable {
        self.waitables[index]
    }

    /// Get the readyness at index
    pub fn is_ready(&self, index: usize) -> bool {
        self.ready.get_bit(index)
    }

    fn ready_resize(&mut self) {
        self.ready.fill(0);
        self.ready.resize(Self::ready_size(self.waitables.len()), 0);
    }

    const fn ready_size(waitables_size: usize) -> usize {
        const BITS: usize = u8::BITS as usize;
        ((waitables_size + BITS - 1) / BITS) * BITS
    }
}

/// Structure of an IPC message
#[derive(Debug)]
pub struct Message {
    /// User data
    ///
    /// May contain type, transaction id, whatever is relevant.
    ///
    /// If data are bigger than 8x8 bytes, you may use shared memory to pass buffer.
    data: MessageData,

    /// Kernel Handles to transmit from one process to another
    ///
    /// From the sender perspective, the handles are sent: they are consumed, they are not valid after the send operation succeeded.
    ///
    /// From the receiver perspective, the handles are owned by the receiver after the receive operation succeeded.
    ///
    /// Set to invalid if no handle
    ///
    handles: [Handle; Self::HANDLE_COUNT],
}

#[derive(Debug)]
#[repr(align(8))]
struct MessageData {
    data: [u8; Message::DATA_SIZE],
}

impl Default for Message {
    fn default() -> Self {
        const INVALID_HANDLE: Handle = Handle::invalid();
        Self {
            data: MessageData {
                data: [0; Self::DATA_SIZE],
            },
            handles: [INVALID_HANDLE; Self::HANDLE_COUNT],
        }
    }
}

impl Message {
    pub const DATA_SIZE: usize = SysMessage::DATA_SIZE * size_of::<u64>();

    pub const HANDLE_COUNT: usize = SysMessage::HANDLE_COUNT;

    /// Construct a new message
    ///
    /// Kernel Handles will be moved into the message, and Handle::invalid() will be left in the slice
    ///
    /// # Safety
    ///
    /// - data must be trivially copiable, with no reference.
    pub unsafe fn new<T: Copy>(data: &T, handles: [Handle; Self::HANDLE_COUNT]) -> Self {
        Self::assert_layout::<T>();

        let mut msg = Message {
            data: MessageData {
                data: [0; Self::DATA_SIZE],
            },
            handles,
        };

        unsafe {
            *msg.data_mut() = *data;
        }

        msg
    }

    unsafe fn from_receive_syscall(sys_msg: SysMessage) -> Message {
        let mut msg = Message::default();
        msg.data.data = unsafe { mem::transmute_copy(&sys_msg.data) };

        for (index, &sys_handle) in sys_msg.handles.iter().enumerate() {
            msg.handles[index] = unsafe { Handle::from_raw(sys_handle) };
        }

        msg
    }

    /// Get a reference to the data
    ///
    /// # Safety
    ///
    /// - The message itself does not enforce type. There is no enforcement that the requested type correspond to the sent type.
    pub unsafe fn data<T>(&self) -> &T {
        Self::assert_layout::<T>();

        unsafe { &*(self.data.data.as_ptr() as *const _) }
    }

    /// Get a mutable reference to the data
    ///
    /// # Safety
    ///
    /// - The message itself does not enforce type. There is no enforcement that the requested type correspond to the sent type.
    pub unsafe fn data_mut<T>(&mut self) -> &mut T {
        Self::assert_layout::<T>();

        unsafe { &mut *(self.data.data.as_mut_ptr() as *mut _) }
    }

    fn assert_layout<T>() {
        assert!(size_of::<T>() <= Self::DATA_SIZE);

        // Alignment is always a power of 2
        // data is `align(8)`
        assert!(mem::align_of::<T>() <= 8);
    }

    /// Get the handle at index (index must be < 8)
    pub fn handle(&self, index: usize) -> &Handle {
        &self.handles[index]
    }

    /// Take the handle at index, leaving Handle::invalid() in the message (index must be < 8)
    pub fn take_handle(&mut self, index: usize) -> Handle {
        let mut handle = Handle::invalid();
        mem::swap(&mut handle, &mut self.handles[index]);
        handle
    }

    pub fn take_all_handles(&mut self) -> [Handle; Self::HANDLE_COUNT] {
        let mut handles: [Handle; Self::HANDLE_COUNT] =
            [const { Handle::invalid() }; Self::HANDLE_COUNT];
        for index in 0..Self::HANDLE_COUNT {
            mem::swap(&mut handles[index], &mut self.handles[index]);
        }
        handles
    }

    fn to_send_syscall(&self) -> SysMessage {
        let mut msg = SysMessage {
            // Note: safe since we kept right alignment
            data: unsafe { mem::transmute(self.data.data) },
            handles: [unsafe { Handle::invalid().as_syscall_value() } as u64;
                Message::HANDLE_COUNT],
        };

        // pass handle values
        for (index, handle) in self.handles.iter().enumerate() {
            if handle.valid() {
                msg.handles[index] = unsafe { handle.as_syscall_value() } as u64;
            }
        }

        msg
    }

    fn after_send_success(&mut self) {
        // On successful syscall, the handle has been MOVED into the message.
        // We must forget them here.

        for handle in self.handles.iter_mut() {
            if handle.valid() {
                // Forget the handle (do NOT close it)
                let mut new_handle = Handle::invalid();
                mem::swap(handle, &mut &mut new_handle);
                mem::forget(new_handle);
            }
        }
    }
}
