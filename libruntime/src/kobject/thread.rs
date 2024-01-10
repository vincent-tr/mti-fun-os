use core::{hint::unreachable_unchecked, mem};

use alloc::boxed::Box;
use libsyscalls::{thread, Handle, Permissions, ThreadPriority};

use super::{Error, KObject, MemoryObject, Process, PAGE_SIZE};

const STACK_SIZE: usize = PAGE_SIZE * 5;

/// Thread
#[derive(Debug)]
pub struct Thread {
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
}

impl Default for ThreadOptions {
    /// Create a new option object with default values
    fn default() -> Self {
        Self {
            stack_size: STACK_SIZE,
            priority: ThreadPriority::Normal,
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
}

impl Thread {
    /// Start a new thread
    pub fn start<Entry: FnOnce() + 'static>(
        entry: Entry,
        options: ThreadOptions,
    ) -> Result<Self, Error> {
        let stack_top = Self::create_stack(options.stack_size)?;

        let arg = Box::new(ThreadParameter::new(entry));
        let arg = Box::leak(arg) as *mut _ as usize;

        // TODO
        let tls = 0;

        let handle = thread::create(
            unsafe { Process::current().handle() },
            options.priority,
            Self::thread_entry,
            stack_top,
            arg,
            tls,
        )
        .map_err(|err| {
            // Thread not created, need to drop args
            let arg = unsafe { Box::from_raw(arg as *mut ThreadParameter) };
            mem::drop(arg);

            err
        })?;

        Ok(Self { handle })
    }

    fn create_stack(size: usize) -> Result<usize, Error> {
        let self_proc = Process::current();

        let stack_reservation = self_proc.map_reserve(None, size + (PAGE_SIZE * 2))?;
        let mobj = MemoryObject::create(size)?;
        let stack = self_proc.map_mem(
            Some(stack_reservation + PAGE_SIZE),
            size,
            Permissions::READ | Permissions::WRITE,
            &mobj,
            0,
        )?;

        Ok(stack + size)
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

        Ok(Self { handle })
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
