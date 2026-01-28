use core::{hint::unreachable_unchecked, mem, ops::Range};

use alloc::{boxed::Box, collections::BTreeMap, format, string::String, sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use libsyscalls::{thread, Handle, HandleType};
use log::debug;
use spin::Mutex;

use super::{tls::TLS_SIZE, *};

pub const STACK_SIZE: usize = PAGE_SIZE * 20;

/// Thread
#[derive(Debug)]
pub struct Thread {
    cached_tid: Mutex<Option<u64>>,
    cached_pid: Mutex<Option<u64>>,
    handle: Handle,
}

impl KObject for Thread {
    unsafe fn handle(&self) -> &Handle {
        &self.handle
    }

    fn into_handle(self) -> Handle {
        self.handle
    }

    unsafe fn from_handle_unchecked(handle: Handle) -> Self {
        Self {
            handle,
            cached_tid: Mutex::new(None),
            cached_pid: Mutex::new(None),
        }
    }

    fn from_handle(handle: Handle) -> Result<Self, Error> {
        if !handle.valid() {
            return Err(Error::InvalidArgument);
        }
        if handle.r#type() != HandleType::Thread {
            return Err(Error::InvalidArgument);
        }

        Ok(unsafe { Self::from_handle_unchecked(handle) })
    }
}

/// Thread options
#[derive(Debug)]
pub struct ThreadOptions<'a> {
    name: Option<&'a str>,
    stack_size: usize,
    priority: ThreadPriority,
    privileged: bool,
}

impl Default for ThreadOptions<'_> {
    /// Create a new option object with default values
    fn default() -> Self {
        Self {
            name: None,
            stack_size: STACK_SIZE,
            priority: ThreadPriority::Normal,
            privileged: false,
        }
    }
}

impl<'a> ThreadOptions<'a> {
    /// Set the name of the future thread
    pub fn name(&mut self, value: &'a str) -> &mut Self {
        self.name = Some(value);
        self
    }

    pub fn clear_name(&mut self) -> &mut Self {
        self.name = None;
        self
    }

    /// Set the size of stack for the future thread
    pub fn stack_size(&mut self, value: usize) -> &mut Self {
        self.stack_size = value;
        self
    }

    /// Set the priority of stack for the future thread
    pub fn priority(&mut self, value: ThreadPriority) -> &mut Self {
        self.priority = value;
        self
    }

    /// Set if the thread runs in privileged mode (ring0)
    ///
    /// # Safety
    ///
    /// Threads in ring0 are very special, must use with care.
    /// For example, if they trigger exception, they will panic() the kernel instead of standard thread error handling.
    pub unsafe fn privileged(&mut self, value: bool) -> &mut Self {
        self.privileged = value;
        self
    }
}

impl Thread {
    /// Start a new thread
    pub fn start<Entry: FnOnce() + 'static>(
        entry: Entry,
        options: ThreadOptions,
    ) -> Result<Self, Error> {
        let stack = AllocWithGuards::new(options.stack_size)?;
        let tls = AllocWithGuards::new(TLS_SIZE)?;
        let mut parameter = Box::new(ThreadParameter::new(entry));

        let arg = parameter.as_mut() as *mut _ as usize;
        let stack_top_addr = stack.address() + options.stack_size;
        let tls_addr = tls.address();

        let handle = thread::create(
            options.name,
            unsafe { Process::current().handle() },
            options.privileged,
            options.priority,
            Self::thread_entry,
            stack_top_addr,
            arg,
            tls_addr,
        )?;

        let stack_reservation = stack.reservation().clone();
        let tls_reservation = tls.reservation().clone();

        // Thread has been created properly, we can leak the allocation
        stack.leak();
        tls.leak();
        Box::leak(parameter);

        let obj = Self {
            cached_tid: Mutex::new(None),
            cached_pid: Mutex::new(None),
            handle,
        };

        THREAD_GC.add_thread(ThreadGCData::new(
            obj.tid(),
            stack_reservation,
            tls_reservation,
        ));

        Ok(obj)
    }

    extern "C" fn thread_entry(arg: usize) -> ! {
        {
            let parameter = unsafe { Box::from_raw(arg as *mut ThreadParameter) };

            (parameter.target)();
        }

        thread::exit().expect("Could not exit thread");
        unsafe { unreachable_unchecked() };
    }

    /// Open the given thread
    pub fn open(tid: u64) -> Result<Self, Error> {
        let handle = thread::open(tid)?;

        Ok(Self {
            cached_tid: Mutex::new(Some(tid)),
            cached_pid: Mutex::new(None),
            handle,
        })
    }

    /// Get the thread id
    pub fn tid(&self) -> u64 {
        if let Some(value) = *self.cached_tid.lock() {
            return value;
        }

        // Will also fill cache
        let info = self.info();
        info.tid
    }

    /// Get the process id of the thread
    pub fn pid(&self) -> u64 {
        if let Some(value) = *self.cached_pid.lock() {
            return value;
        }

        // Will also fill cache
        let info = self.info();
        info.pid
    }

    /// Set thread priority
    pub fn set_priority(&self, priority: ThreadPriority) -> Result<(), Error> {
        thread::set_priority(&self.handle, priority)?;

        Ok(())
    }

    /// Get thread info
    pub fn info(&self) -> ThreadInfo {
        let info = thread::info(&self.handle).expect("Could not get thread info");

        {
            let mut cached_tid = self.cached_tid.lock();

            if cached_tid.is_none() {
                *cached_tid = Some(info.tid);
            }
        }

        {
            let mut cached_pid = self.cached_pid.lock();

            if cached_pid.is_none() {
                *cached_pid = Some(info.pid);
            }
        }

        info
    }

    /// List the thread ids in the system
    pub fn list() -> Result<Box<[u64]>, Error> {
        let mut size = 1024;

        // Event not atomic, let's consider that with doubling the required size between call,
        // at some point we will be able to fetch list entirely
        loop {
            let mut buffer = Vec::with_capacity(size);
            buffer.resize(size, 0);

            let (_, new_size) = thread::list(&mut buffer)?;

            if new_size > size {
                // Retry with 2x requested size
                size = new_size * 2;
                continue;
            }

            buffer.resize(new_size, 0);

            return Ok(buffer.into_boxed_slice());
        }
    }

    /// Set the name of the thread
    pub fn set_name(&self, name: &str) -> Result<(), Error> {
        thread::set_name(&self.handle, name)?;

        Ok(())
    }

    /// Get the name of the thread
    pub fn name(&self) -> Result<String, Error> {
        let mut size = ThreadInfo::NAME_LEN;

        // Even if not atomic, let's consider we won't have many tries before we get a correct size
        loop {
            let mut buffer = Vec::with_capacity(size);
            buffer.resize(size, 0);

            let (_, new_size) = thread::get_name(&self.handle, &mut buffer)?;

            if new_size > size {
                // Retry
                size = new_size;
                continue;
            }

            buffer.resize(new_size, 0);

            return Ok(unsafe { String::from_utf8_unchecked(buffer) });
        }
    }

    /// Kill the target thread
    ///
    /// # Safety
    /// - the objects on the local stack won't be dropped. The stack memory will be freed without executing destructors
    /// - the TLS objects won't be dropped. The TLS slots memory of the thread will be freed without executing descrutors
    pub unsafe fn kill(&self) -> Result<(), Error> {
        thread::kill(&self.handle)?;

        Ok(())
    }
}

pub struct AllocWithGuards<'a> {
    reservation: Mapping<'a>,
}

impl<'a> AllocWithGuards<'a> {
    pub fn new_remote(size: usize, process: &'a Process) -> Result<Self, Error> {
        let reservation = process.map_reserve(None, size + (PAGE_SIZE * 2))?;
        let addr = reservation.address() + PAGE_SIZE;

        let mobj = MemoryObject::create(size)?;

        let mapping = process.map_mem(
            Some(addr),
            size,
            Permissions::READ | Permissions::WRITE,
            &mobj,
            0,
        )?;

        // Note: we can safely free the real mapping since the reservation is a superset of it.
        // Droppping the reservation will drop the mapping as well
        mapping.leak();

        Ok(Self { reservation })
    }

    pub fn new(size: usize) -> Result<Self, Error> {
        Self::new_remote(size, &Process::current())
    }

    pub fn address(&self) -> usize {
        self.reservation.address() + PAGE_SIZE
    }

    pub fn reservation(&self) -> &Range<usize> {
        self.reservation.range()
    }

    /// Leak the allocation.
    ///
    /// Consume the current object without freeing the allocated memory
    pub fn leak(self) {
        self.reservation.leak()
    }
}

/// Supervisor for a thread
#[derive(Debug)]
pub struct ThreadSupervisor<'a> {
    target: &'a Thread,
}

impl<'a> ThreadSupervisor<'a> {
    /// Construct a new thread supervisor
    pub fn new(target: &'a Thread) -> Self {
        Self { target }
    }

    /// Get a reference to the target thread
    pub fn target(&self) -> &'a Thread {
        &self.target
    }

    /// When the thread is in error state, get the error details
    pub fn error_info(&self) -> Result<Exception, Error> {
        let info = thread::error_info(unsafe { &self.target.handle() })?;

        Ok(info)
    }

    /// When the thread is in error state, resume it
    pub fn resume(&self) -> Result<(), Error> {
        thread::resume(unsafe { &self.target.handle() })?;

        Ok(())
    }

    /// When the thread is in error state, get its context
    pub fn context(&self) -> Result<ThreadContext, Error> {
        let context = thread::context(unsafe { &self.target.handle() })?;

        Ok(context)
    }

    /// When the thread is in error state, update its context
    pub fn update_context(&self, regs: &[(ThreadContextRegister, usize)]) -> Result<(), Error> {
        thread::update_context(unsafe { &self.target.handle() }, regs)?;

        Ok(())
    }
}

struct ThreadParameter {
    target: Box<dyn FnOnce()>,
}

impl ThreadParameter {
    pub fn new<Entry: FnOnce() + 'static>(entry: Entry) -> Self {
        Self {
            target: Box::new(entry),
        }
    }
}

/// Cleanup TLS and Stack allocated for threads
pub struct ThreadGC {
    exit_port: Mutex<Option<PortSender>>,
    data: Arc<Mutex<BTreeMap<u64, ThreadGCData>>>,
}

lazy_static! {
    pub static ref THREAD_GC: ThreadGC = ThreadGC::new();
}

impl ThreadGC {
    /// Construct a new GC
    pub fn new() -> Self {
        Self {
            exit_port: Mutex::new(None),
            data: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Start the GC thread
    pub fn init(&self) {
        let (exit_receiver, exit_port) = Port::create(None).expect("Could not create port");
        *self.exit_port.lock() = Some(exit_port);

        let data = self.data.clone();

        let mut options = ThreadOptions::default();
        options.name("thread-gc");
        options.priority(ThreadPriority::AboveNormal);

        let entry = move || Self::worker(exit_receiver, data);

        Thread::start(entry, options).expect("Could not start thread");
    }

    /// Exit the GC thread
    pub fn terminate(&self) {
        let mut message = Message::default();
        let mut exit_port = self.exit_port.lock();

        exit_port
            .take()
            .expect("Could not get exit port")
            .send(&mut message)
            .expect("Could not exit thread-gc");
    }

    fn add_thread(&self, item: ThreadGCData) {
        let mut data = self.data.lock();

        data.insert(item.tid, item);
    }

    fn worker(exit: PortReceiver, data: Arc<Mutex<BTreeMap<u64, ThreadGCData>>>) {
        let pid = Process::current().pid();

        let listener = ThreadListener::create(ThreadListenerFilter::Pids(&[pid]))
            .expect("failed to create thread listener");

        let mut waiter = Waiter::new(&[&exit, &listener]);

        loop {
            waiter.wait().expect("wait failed");
            if waiter.is_ready(0) {
                break;
            }

            assert!(waiter.is_ready(1));

            let event = listener.receive().expect("could not read listener");

            if let ThreadEventType::Terminated = event.r#type {
                // Thread terminated. Can reclaim its stack/TLS
                // Note that if the thread has been remotely created, we have no info on this
                let mut data = data.lock();

                if let Some(item) = data.remove(&event.tid) {
                    debug!(
                        "Dropping tid={}: stack reservation=[0x{:016X} - 0x{:016X}], TLSreservation=[0x{:016X} - 0x{:016X}]",
                        event.tid,
                        item.stack_reservation.start,
                        item.stack_reservation.end,
                        item.tls_reservation.start,
                        item.tls_reservation.end
                    );

                    // Explicit
                    mem::drop(item);
                }
            }
        }
    }
}

#[derive(Debug)]
struct ThreadGCData {
    tid: u64,
    stack_reservation: Range<usize>,
    tls_reservation: Range<usize>,
}

impl ThreadGCData {
    pub fn new(tid: u64, stack_reservation: Range<usize>, tls_reservation: Range<usize>) -> Self {
        Self {
            tid,
            stack_reservation,
            tls_reservation,
        }
    }
}

impl Drop for ThreadGCData {
    fn drop(&mut self) {
        let process = Process::current();

        process
            .unmap(&self.stack_reservation)
            .expect(&format!("Could not free stack for thread {}", self.tid));
        process
            .unmap(&self.tls_reservation)
            .expect(&format!("Could not free tls for thread {}", self.tid));
    }
}
