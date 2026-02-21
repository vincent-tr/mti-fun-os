use alloc::sync::Arc;
use core::arch::asm;
use libruntime::kobject::{
    self, Exception, PAGE_SIZE, Permissions, ThreadContextRegister, ThreadEventType,
    ThreadListenerFilter, ThreadOptions, Timer, TlsAllocator,
};
use log::{debug, info};

/// Dump all processes and threads in the system
#[allow(dead_code)]
pub fn dump_processes_threads() {
    let pids = kobject::Process::list().expect("Could not list pids");
    info!("pids list = {:?}", pids);

    for &pid in pids.iter() {
        let process = kobject::Process::open(pid).expect("Could not open pid");
        info!("  {:?}", process.info());
        info!(
            "  name={}",
            process.name().expect("Could not get process name")
        );
    }

    let tids = kobject::Thread::list().expect("Could not list tids");
    info!("tids list = {:?}", tids);

    for &tid in tids.iter() {
        let thread = kobject::Thread::open(tid).expect("Could not open tid");
        info!("  {:?}", thread.info());
        info!(
            "  name={}",
            thread.name().expect("Could not get thread name")
        );
    }
}

/// Test thread event listener (debug breaks and page faults)
#[allow(dead_code)]
pub fn listen_threads() {
    let slot = Arc::new(TlsAllocator::allocate().expect("Could not allocate tls slot"));

    let cloned_slot = slot.clone();
    let debugbreak = || {
        let slot = cloned_slot;

        assert!(slot.get().is_none());
        slot.set(42);

        let mut value = 42;
        unsafe {
            asm!("int3", inlateout("rax") value => value, options(nostack, preserves_flags));
        }

        debug!("resumed (value={value})");
        debug!("tls={}", slot.get().unwrap_or(0));
    };

    const PAGE_FAULT_ADDR: usize = 0x1000000;

    let cloned_slot = slot.clone();
    let pagefault = || {
        let slot = cloned_slot;

        assert!(slot.get().is_none());
        slot.set(43);

        let ptr = PAGE_FAULT_ADDR as *mut u8;
        unsafe { *ptr = 42 };

        debug!("resumed");
        debug!("tls={}", slot.get().unwrap_or(0));
    };

    let listen = move || {
        let listener = kobject::ThreadListener::create(ThreadListenerFilter::All)
            .expect("failed to create thread listener");

        // Keep thread handle alive
        let mut options = ThreadOptions::default();
        options.name("debugbreak");
        let thread_debugbreak =
            kobject::Thread::start(debugbreak, options).expect("could not create thread");

        // Keep thread handle alive
        let mut options = ThreadOptions::default();
        options.name("pagefault");
        let thread_pagefault =
            kobject::Thread::start(pagefault, options).expect("could not create thread");

        debug!("debugbreak_tid = {}", thread_debugbreak.tid());
        debug!("pagefault_tid = {}", thread_pagefault.tid());

        loop {
            let event = listener.blocking_receive().expect("receive failed");

            debug!("Thread event: {:?}", event);

            if let ThreadEventType::Error = event.r#type {
                let thread = if event.tid == thread_debugbreak.tid() {
                    &thread_debugbreak
                } else if event.tid == thread_pagefault.tid() {
                    &thread_pagefault
                } else {
                    panic!("unexpected error");
                };

                let supervisor = kobject::ThreadSupervisor::new(thread);

                let err = supervisor
                    .error_info()
                    .expect("could not get thread error info");

                debug!("Thread error: {:?} in thread {}", err, event.tid);

                match err {
                    Exception::Breakpoint => {
                        // change context: update rax
                        let context = supervisor.context().expect("get context failed");
                        debug!("Thread RAX = {}", context.rax);
                        supervisor
                            .update_context(&[(ThreadContextRegister::Rax, context.rax + 1)])
                            .expect("set context failed");

                        debug!("Thread resume");
                        supervisor.resume().expect("resume failed");
                    }
                    Exception::PageFault(_error_code, address) => {
                        let self_proc = kobject::Process::current();
                        let page = kobject::MemoryObject::create(PAGE_SIZE)
                            .expect("Could not create page");

                        let mapping = self_proc
                            .map_mem(
                                Some(address),
                                PAGE_SIZE,
                                Permissions::READ | Permissions::WRITE,
                                &page,
                                0,
                            )
                            .expect("Could not map page");
                        mapping.leak(); // only for testing purposes

                        debug!("Thread resume");
                        supervisor.resume().expect("resume failed");
                    }
                    _ => {}
                }

                // thread handle will be dropped here
            }
        }
    };

    let mut options = ThreadOptions::default();
    options.name("thread-listener");
    kobject::Thread::start(listen, options).expect("Could not create listen thread");
}

/// Test timer with 1 second interval
#[allow(dead_code)]
pub fn interval_second() {
    let timer = Timer::create(42).expect("failed to create timer");

    const DELAY: u64 = 1_000_000_000; // 1 second in nanoseconds

    loop {
        let now = Timer::now().expect("failed to get current time");
        timer.arm(now + DELAY).expect("failed to arm timer");
        let msg = timer
            .blocking_receive()
            .expect("failed to receive timer event");
        info!("tick armed at {}, fired at {}", now, msg.now);
    }
}
