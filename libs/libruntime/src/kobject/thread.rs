use core::{fmt, hint::unreachable_unchecked, mem, ops::Range};

use alloc::{boxed::Box, collections::BTreeMap, format, string::String, sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use libsyscalls::{Handle, HandleType, thread};
use log::debug;
use spin::Mutex;

use crate::debug::StackTrace;

use super::{tls::TLS_SIZE, *};

pub const STACK_SIZE: usize = PAGE_SIZE * 20;

pub(crate) unsafe fn init() {
    unsafe {
        ThreadRuntime::get().init();
    }
}

pub unsafe fn exit() {
    unsafe {
        ThreadRuntime::get().exit();
    }
}

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

impl Clone for Thread {
    fn clone(&self) -> Self {
        let handle = self.handle.clone();

        Self {
            handle,
            cached_tid: Mutex::new(*self.cached_tid.lock()),
            cached_pid: Mutex::new(*self.cached_pid.lock()),
        }
    }
}

/// Thread options
#[derive(Debug)]
pub struct ThreadOptions<'a> {
    name: Option<&'a str>,
    stack_size: usize,
    priority: ThreadPriority,
    privileged: bool,
    suspended: bool,
}

impl Default for ThreadOptions<'_> {
    /// Create a new option object with default values
    fn default() -> Self {
        Self {
            name: None,
            stack_size: STACK_SIZE,
            priority: ThreadPriority::Normal,
            privileged: false,
            suspended: false,
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

    /// Set if the thread starts in a suspended state
    pub fn suspended(&mut self, value: bool) -> &mut Self {
        self.suspended = value;
        self
    }
}

impl Thread {
    /// Create a new thread
    pub fn create<Entry: FnOnce() + 'static>(
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
            true,
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

        ThreadRuntime::get().add_thread(ThreadRuntimeData::new(
            obj.tid(),
            obj.clone(),
            stack_reservation,
            tls_reservation,
        ));

        // Now that all is ready, we can start the thread if needed
        if !options.suspended {
            obj.resume().expect("Could not resume thread");
        }

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

    pub fn open_self() -> Result<Self, Error> {
        let handle = thread::open_self()?;

        Ok(Self {
            cached_tid: Mutex::new(None),
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

    /// Resume the thread
    ///
    /// This can be called for 2 reasons:
    /// - the thread was created in suspended state
    /// - the thread is in error state and we want to resume it
    pub fn resume(&self) -> Result<(), Error> {
        thread::resume(&self.handle)?;

        Ok(())
    }

    /// Kill the target thread
    ///
    /// # Safety
    /// - the objects on the local stack won't be dropped. The stack memory will be freed without executing destructors
    /// - the TLS objects won't be dropped. The TLS slots memory of the thread will be freed without executing destructors
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
        self.target.resume()?;

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
pub struct ThreadRuntime {
    exit_port: Mutex<Option<PortSender>>,
    data: Arc<Mutex<BTreeMap<u64, ThreadRuntimeData>>>,
}

impl ThreadRuntime {
    /// Get the global thread runtime instance
    pub fn get() -> &'static Self {
        lazy_static! {
            static ref INSTANCE: ThreadRuntime = ThreadRuntime::new();
        }

        &INSTANCE
    }

    fn new() -> Self {
        Self {
            exit_port: Mutex::new(None),
            data: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Intialize the thread runtime.
    ///
    /// # Safety
    ///
    /// - must be called only once, from the main thread. Calling it multiple times or from other threads may cause undefined behavior.
    unsafe fn init(&self) {
        // Start maintenance thread
        let (exit_receiver, exit_port) = Port::create(None).expect("Could not create port");
        *self.exit_port.lock() = Some(exit_port);

        let data = self.data.clone();

        let mut options = ThreadOptions::default();
        options.name("thread-runtime");
        options.priority(ThreadPriority::AboveNormal);

        let entry = move || Self::worker(exit_receiver, data);

        Thread::create(entry, options).expect("Could not start thread");

        // Register self main thread (created by our creator)
        let main_thread = Thread::open_self().expect("Could not open self thread");
        let main_thread_data = ThreadRuntimeData::new(
            main_thread.tid(),
            main_thread,
            // No stack or TLS to free for main thread, its exit will exit the whole process
            0..0,
            0..0,
        );

        self.add_thread(main_thread_data);
    }

    /// Exit the thread runtime
    ///
    /// # Safety
    ///
    /// - must be called only once, from the main thread. Calling it multiple times or from other threads may cause undefined behavior.
    unsafe fn exit(&self) {
        let mut message = Message::default();
        let mut exit_port = self.exit_port.lock();

        exit_port
            .take()
            .expect("Could not get exit port")
            .send(&mut message)
            .expect("Could not exit thread-runtime");
    }

    fn add_thread(&self, item: ThreadRuntimeData) {
        let mut data = self.data.lock();

        data.insert(item.tid, item);
    }

    fn worker(exit: PortReceiver, data: Arc<Mutex<BTreeMap<u64, ThreadRuntimeData>>>) {
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

            match event.r#type {
                ThreadEventType::Terminated => Self::thread_terminated(event.tid, &data),
                ThreadEventType::Error => Self::thread_error(event.tid, &data),
                _ => {}
            }
        }
    }

    fn thread_terminated(tid: u64, data: &Mutex<BTreeMap<u64, ThreadRuntimeData>>) {
        // Thread terminated. Can reclaim its stack/TLS
        // Note that if the thread has been remotely created, we have no info on this
        let mut data = data.lock();

        if let Some(item) = data.remove(&tid) {
            debug!(
                "Dropping tid={}: stack reservation=[0x{:016X} - 0x{:016X}], TLSreservation=[0x{:016X} - 0x{:016X}]",
                tid,
                item.stack_reservation.start,
                item.stack_reservation.end,
                item.tls_reservation.start,
                item.tls_reservation.end
            );

            // Explicit
            mem::drop(item);
        }
    }

    fn thread_error(tid: u64, data: &Mutex<BTreeMap<u64, ThreadRuntimeData>>) -> ! {
        use alloc::fmt::Write;

        let data = data.lock();
        let thread_data = data.get(&tid).expect(&format!(
            "Thread in error but not found in runtime data (tid={})",
            tid
        ));

        let supervisor = ThreadSupervisor::new(&thread_data.handle);
        let error = supervisor.error_info().expect("Could not get error info");
        let context = supervisor.context().expect("Could not get thread context");

        let mut msg = String::new();

        writeln!(&mut msg, "Thread {} error", tid,).unwrap();

        writeln!(&mut msg, "\n----------\n").unwrap();

        writeln!(&mut msg, "{}", ExceptionFormatter(&error)).unwrap();

        writeln!(&mut msg, "\nRegisters:").unwrap();
        writeln!(
            &mut msg,
            "  rax: {:#018x}  rbx: {:#018x}  rcx: {:#018x}  rdx: {:#018x}",
            context.rax, context.rbx, context.rcx, context.rdx
        )
        .unwrap();
        writeln!(
            &mut msg,
            "  rsi: {:#018x}  rdi: {:#018x}  rbp: {:#018x}  rsp: {:#018x}",
            context.rsi, context.rdi, context.rbp, context.rsp
        )
        .unwrap();
        writeln!(
            &mut msg,
            "  r8:  {:#018x}  r9:  {:#018x}  r10: {:#018x}  r11: {:#018x}",
            context.r8, context.r9, context.r10, context.r11
        )
        .unwrap();
        writeln!(
            &mut msg,
            "  r12: {:#018x}  r13: {:#018x}  r14: {:#018x}  r15: {:#018x}",
            context.r12, context.r13, context.r14, context.r15
        )
        .unwrap();
        writeln!(
            &mut msg,
            "  rip: {:#018x}  flags: {:#018x}  tls: {:#018x}",
            context.instruction_pointer, context.cpu_flags, context.tls
        )
        .unwrap();

        writeln!(
            &mut msg,
            "\nStack bounds: [{:#018x} - {:#018x}] (size: {} bytes)",
            thread_data.stack_reservation.start,
            thread_data.stack_reservation.end,
            thread_data.stack_reservation.end - thread_data.stack_reservation.start
        )
        .unwrap();

        let stacktrace = StackTrace::capture_from_context(&context);
        writeln!(&mut msg, "\nStacktrace:").unwrap();

        for frame in stacktrace.iter() {
            if let Some(info) = frame.location() {
                writeln!(
                    &mut msg,
                    "  at {} +{}",
                    info.function_name(),
                    info.function_offset()
                )
                .unwrap();
            } else {
                writeln!(&mut msg, "  at 0x{0:016X}", frame.address()).unwrap();
            }
        }

        writeln!(&mut msg, "\n----------\n").unwrap();

        // Panic on thread error so that the whole process is terminated.
        panic!("{}", msg);
    }
}

#[derive(Debug)]
struct ThreadRuntimeData {
    tid: u64,
    handle: Thread,
    stack_reservation: Range<usize>,
    tls_reservation: Range<usize>,
}

impl ThreadRuntimeData {
    pub fn new(
        tid: u64,
        handle: Thread,
        stack_reservation: Range<usize>,
        tls_reservation: Range<usize>,
    ) -> Self {
        Self {
            tid,
            handle,
            stack_reservation,
            tls_reservation,
        }
    }
}

impl Drop for ThreadRuntimeData {
    fn drop(&mut self) {
        let process = Process::current();

        // Empty reservation can happen for remotely created threads, for which we have no info on stack/TLS
        // This normally occur only on the main thread, since all other threads are created through our API, and the main thread exits, so it should not happen
        //
        // One notable case is the first init thread created by the kernel. It has no freeable stack (part of the binary image), and no TLS (since it's created before we init the TLS allocator).

        if !self.stack_reservation.is_empty() {
            process
                .unmap(&self.stack_reservation)
                .expect(&format!("Could not free stack for thread {}", self.tid));
        }

        if !self.tls_reservation.is_empty() {
            process
                .unmap(&self.tls_reservation)
                .expect(&format!("Could not free tls for thread {}", self.tid));
        }
    }
}

/// Format an exception with detailed human-readable information
struct ExceptionFormatter<'a>(&'a Exception);

impl<'a> fmt::Display for ExceptionFormatter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Exception::DivideError => write!(f, "Divide by zero error"),
            Exception::Debug => write!(f, "Debug exception"),
            Exception::NonMaskableInterrupt => write!(f, "Non-maskable interrupt"),
            Exception::Breakpoint => write!(f, "Breakpoint"),
            Exception::Overflow => write!(f, "Overflow"),
            Exception::BoundRangeExceeded => write!(f, "Bound range exceeded"),
            Exception::InvalidOpcode => write!(f, "Invalid opcode"),
            Exception::DeviceNotAvailable => write!(f, "Device not available"),
            Exception::DoubleFault => write!(f, "Double fault"),
            Exception::InvalidTSS => write!(f, "Invalid TSS"),
            Exception::SegmentNotPresent(code) => {
                write!(f, "Segment not present (error code: {:#x})", code)
            }
            Exception::StackSegmentFault(code) => {
                write!(f, "Stack segment fault (error code: {:#x})", code)
            }
            Exception::GeneralProtectionFault(code) => {
                write!(f, "General protection fault (error code: {:#x})", code)
            }
            Exception::PageFault(error_code, address) => {
                write!(f, "Page fault at address {:#x}: ", address)?;

                // Decode error code bits
                let present = (*error_code & 0x1) != 0;
                let write = (*error_code & 0x2) != 0;
                let user = (*error_code & 0x4) != 0;
                let malformed_table = (*error_code & 0x8) != 0;
                let instruction = (*error_code & 0x10) != 0;

                if present {
                    write!(f, "protection violation")?;
                } else {
                    write!(f, "page not present")?;
                }

                if write {
                    write!(f, " during write")?;
                } else if instruction {
                    write!(f, " during instruction fetch")?;
                } else {
                    write!(f, " during read")?;
                }

                if user {
                    write!(f, " in user mode")?;
                } else {
                    write!(f, " in supervisor mode")?;
                }

                if malformed_table {
                    write!(f, " (malformed table)")?;
                }

                write!(f, " (error code: {:#x})", error_code)
            }
            Exception::X87FloatingPoint => write!(f, "x87 floating point exception"),
            Exception::AlignmentCheck => write!(f, "Alignment check"),
            Exception::MachineCheck => write!(f, "Machine check"),
            Exception::SimdFloatingPoint => write!(f, "SIMD floating point exception"),
            Exception::Virtualization => write!(f, "Virtualization exception"),
            Exception::CpProtectionException(code) => {
                write!(f, "Control protection exception (error code: {:#x})", code)
            }
            Exception::HvInjectionException => write!(f, "Hypervisor injection exception"),
            Exception::VmmCommunicationException(code) => {
                write!(f, "VMM communication exception (error code: {:#x})", code)
            }
            Exception::SecurityException(code) => {
                write!(f, "Security exception (error code: {:#x})", code)
            }
        }
    }
}
