use core::hint::unreachable_unchecked;

use alloc::boxed::Box;
use libsyscalls::{thread, Handle};
use spin::Mutex;

use super::{tls::TLS_SIZE, *};

const STACK_SIZE: usize = PAGE_SIZE * 5;

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
}

/// Thread options
#[derive(Debug)]
pub struct ThreadOptions {
    stack_size: usize,
    priority: ThreadPriority,
    privileged: bool,
}

impl Default for ThreadOptions {
    /// Create a new option object with default values
    fn default() -> Self {
        Self {
            stack_size: STACK_SIZE,
            priority: ThreadPriority::Normal,
            privileged: false,
        }
    }
}

impl ThreadOptions {
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
            unsafe { Process::current().handle() },
            options.privileged,
            options.priority,
            Self::thread_entry,
            stack_top_addr,
            arg,
            tls_addr,
        )?;

        // Thread has been created properly, we can leak the allocation
        stack.leak();
        tls.leak();
        Box::leak(parameter);

        Ok(Self {
            cached_tid: Mutex::new(None),
            cached_pid: Mutex::new(None),
            handle,
        })
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
        thread::set_priority(&self.handle, priority)
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
}

struct AllocWithGuards<'a> {
    reservation: Mapping<'a>,
}

impl AllocWithGuards<'_> {
    pub fn new(size: usize) -> Result<Self, Error> {
        let self_proc = Process::current();

        let reservation = self_proc.map_reserve(None, size + (PAGE_SIZE * 2))?;
        let addr = reservation.address() + PAGE_SIZE;

        let mobj = MemoryObject::create(size)?;

        let mapping = self_proc.map_mem(
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

    pub fn address(&self) -> usize {
        self.reservation.address() + PAGE_SIZE
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
        thread::error_info(unsafe { &self.target.handle() })
    }

    /// When the thread is in error state, resume it
    pub fn resume(&self) -> Result<(), Error> {
        thread::resume(unsafe { &self.target.handle() })
    }

    /// When the thread is in error state, get its context
    pub fn context(&self) -> Result<ThreadContext, Error> {
        thread::context(unsafe { &self.target.handle() })
    }

    /// When the thread is in error state, update its context
    pub fn update_context(&self, regs: &[(ThreadContextRegister, usize)]) -> Result<(), Error> {
        thread::update_context(unsafe { &self.target.handle() }, regs)
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
