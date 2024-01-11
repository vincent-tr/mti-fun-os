#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;

mod offsets;

use core::{arch::asm, hint::unreachable_unchecked};

use libruntime::kobject::{
    self, Exception, Handle, Permissions, ThreadContextRegister, ThreadEventType, ThreadOptions,
    ThreadPriority, PAGE_SIZE,
};
use libsyscalls::thread;
use log::{debug, info};

// Special init start: need to setup its own stack
#[naked]
#[no_mangle]
#[link_section = ".text_entry"]
pub unsafe extern "C" fn user_start() {
    asm!(
        "
        lea rsp, {stack}
        mov rbp, rsp

        call {main}
        # `main` must never return.
        ud2
        ",
        stack = sym offsets::__init_stack_end,
        main = sym main,
        options(noreturn),
    );
}

// Force at least one data, so that it is laid out after bss in linker script
// This force bss allocation in binary file
#[used(linker)]
static mut FORCE_DATA_SECTION: u8 = 0x42;

extern "C" fn main() -> ! {
    libruntime::init();

    let self_proc = libsyscalls::process::open_self().expect("Could not open self process");

    apply_memory_protections(&self_proc);

    create_idle_task();

    dump_processes_threads();

    listen_threads();

    do_ipc();

    //debug!("Exiting");
    //process::exit().expect("Could not exit process");
    thread::exit().expect("Could not exit thread");
    unsafe { unreachable_unchecked() };
}

fn create_idle_task() {
    let mut options = ThreadOptions::default();

    // Small stack, does not do much
    options.stack_size(PAGE_SIZE);
    options.priority(ThreadPriority::Idle);

    let idle = || {
        // TODO: better sleep
        loop {}
    };

    kobject::Thread::start(idle, options).expect("Could not create idle task");
}

fn apply_memory_protections(self_proc: &Handle) {
    let text_range = offsets::text();
    let rodata_range = offsets::rodata();
    let data_range = offsets::data();

    libsyscalls::process::mprotect(
        &self_proc,
        &text_range,
        Permissions::READ | Permissions::EXECUTE,
    )
    .expect("Could not setup memory protection");

    libsyscalls::process::mprotect(&self_proc, &rodata_range, Permissions::READ)
        .expect("Could not setup memory protection");

    libsyscalls::process::mprotect(
        &self_proc,
        &data_range,
        Permissions::READ | Permissions::WRITE,
    )
    .expect("Could not setup memory protection");

    debug!(
        "text: 0x{:016X} -> 0x{:016X} (size=0x{:X})",
        text_range.start,
        text_range.end,
        text_range.len()
    );
    debug!(
        "rodata: 0x{:016X} -> 0x{:016X} (size=0x{:X})",
        rodata_range.start,
        rodata_range.end,
        rodata_range.len()
    );
    debug!(
        "data: 0x{:016X} -> 0x{:016X} (size=0x{:X})",
        data_range.start,
        data_range.end,
        data_range.len()
    );
    debug!("stack_top: 0x{:016X}", offsets::stack_top());
}

fn dump_processes_threads() {
    let mut pids_buff: [u64; 32] = [0; 32];
    let (pids, count) = libsyscalls::process::list(&mut pids_buff).expect("Could not list pids");
    info!("pids list = {:?} (count={})", pids, count);

    for &pid in pids {
        let process = libsyscalls::process::open(pid).expect("Could not open pid");
        let info = libsyscalls::process::info(&process).expect("Could not get process info");
        info!("  {:?}", info);
    }

    let mut tids_buff: [u64; 32] = [0; 32];
    let (tids, count) = libsyscalls::thread::list(&mut tids_buff).expect("Could not list tids");
    info!("tids list = {:?} (count={})", tids, count);

    for &tid in tids {
        let thread = libsyscalls::thread::open(tid).expect("Could not open tid");
        let info = libsyscalls::thread::info(&thread).expect("Could not get thread info");
        info!("  {:?}", info);
    }
}

fn do_ipc() {
    // create thread, send data and wait back

    let (echo_reader, main_sender) = kobject::Port::create(None).expect("failed to create ipc");
    let (main_reader, echo_sender) = kobject::Port::create(None).expect("failed to create ipc");

    let echo = move || {
        let mut message = echo_reader.blocking_receive().expect("receive failed");
        echo_sender.send(&mut message).expect("send failed");
    };

    kobject::Thread::start(echo, ThreadOptions::default()).expect("could not create echo thread");

    let mut msg = unsafe { kobject::Message::new::<i32>(&42, &mut []) };
    main_sender.send(&mut msg).expect("send failed");

    let msg = main_reader.blocking_receive().expect("wait failed");

    assert!(unsafe { *msg.data::<i32>() } == 42);
    debug!("IPC ALL GOOD");
}

fn listen_threads() {
    let debugbreak = || {
        unsafe {
            set_tls(42);
        }

        let mut value = 42;
        unsafe {
            asm!("int3", inlateout("rax") value => value, options(nostack, preserves_flags));
        }

        debug!("debugbreak: resumed (value={value})");
        debug!("debugbreak: tls={}", unsafe { get_tls() });
    };

    const PAGE_FAULT_ADDR: usize = 0x1000000;

    let pagefault = || {
        unsafe {
            set_tls(43);
        }

        let ptr = PAGE_FAULT_ADDR as *mut u8;
        unsafe { *ptr = 42 };

        debug!("page_fault: resumed");
        debug!("page_fault: tls={}", unsafe { get_tls() });
    };

    let listen = move || {
        let listener =
            kobject::ThreadListener::create(None).expect("failed to create thread listener");

        // Keep thread handle alive
        let thread_debugbreak = kobject::Thread::start(debugbreak, ThreadOptions::default())
            .expect("could not create thread");

        // Keep thread handle alive
        let thread_pagefault = kobject::Thread::start(pagefault, ThreadOptions::default())
            .expect("could not create thread");

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
                        libsyscalls::process::open_self().expect("Could not open self process");
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

    kobject::Thread::start(listen, ThreadOptions::default())
        .expect("Could not create listen thread");
}

// Helpers

unsafe fn set_tls(value: usize) {
    asm!("mov fs:[0], {value};", value = in(reg)value, options(nostack, preserves_flags));
}

unsafe fn get_tls() -> usize {
    let mut value: usize;
    asm!("mov {value}, fs:[0];", value = out(reg)value, options(nostack, preserves_flags));
    value
}
