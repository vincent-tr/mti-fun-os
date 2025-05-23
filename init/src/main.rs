#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;

mod archive;
mod idle;
mod loader;
mod offsets;

use core::{
    arch::{asm, naked_asm},
    hint::unreachable_unchecked,
    ops::Range,
    slice,
};

use alloc::sync::Arc;
use libruntime::{
    debug,
    kobject::{
        self, Exception, Permissions, ThreadContextRegister, ThreadEventType, ThreadListenerFilter,
        ThreadOptions, TlsAllocator, PAGE_SIZE,
    },
};
use log::{debug, info};

// Special init start: need to setup its own stack
#[naked]
#[no_mangle]
#[link_section = ".text_entry"]
pub unsafe extern "C" fn user_start() -> ! {
    naked_asm!(
        "
        lea rsp, {stack}

        call {entry}
        # `entry` must never return.
        ud2
        ",
        stack = sym offsets::__init_stack_end,
        entry = sym entry,
    );
}

extern "C" fn entry(binary_len: usize) -> ! {
    let binary = unsafe { slice::from_raw_parts(offsets::global().start as *const u8, binary_len) };
    libruntime::debug::init_memory_binary(binary);

    libruntime::init();

    apply_memory_protections(binary_len);

    // Jump to a safer thread, with better stack
    let mut options = ThreadOptions::default();
    options.name("main");
    kobject::Thread::start(main, options).expect("Could not start main thread");

    libsyscalls::thread::exit().expect("Failed to exit thread");
    unsafe { unreachable_unchecked() };
}

fn main() {
    idle::create_idle_process().expect("Could not create idle process");

    // dump_processes_threads();
    // listen_threads();
    // do_ipc();
    // kmem_stats();
    // test_unwind();

    loader::load(archive::LIBRUNTIME).expect("Could not load runtime library");
    loader::load(archive::PROCESS_SERVER).expect("Could not load process server");

    libruntime::exit();
}

#[allow(dead_code)]
#[inline(never)]
fn test_unwind() {
    test_unwind2();
}

#[inline(never)]
fn test_unwind2() {
    test_unwind3();
}

#[inline(never)]
fn test_unwind3() {
    panic!("test unwind");
}

fn apply_memory_protections(binary_len: usize) {
    setup_protection(
        "text",
        offsets::text(),
        Permissions::READ | Permissions::EXECUTE,
    );

    setup_protection("rodata", offsets::rodata(), Permissions::READ);

    setup_protection(
        "data",
        offsets::data(),
        Permissions::READ | Permissions::WRITE,
    );

    let unmapped_range_start = offsets::global().end;
    let unmapped_range_end = offsets::global().start + binary_len;
    // Align
    let unmapped_range =
        unmapped_range_start..(((unmapped_range_end + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE);

    setup_protection("unmapped", unmapped_range, Permissions::READ);

    fn setup_protection(name: &str, range: Range<usize>, perms: Permissions) {
        // kernel has mapped one area with all permissions set
        let initial_perms = Permissions::READ | Permissions::WRITE | Permissions::EXECUTE;
        let process = kobject::Process::current();

        unsafe {
            let mut mapping = kobject::Mapping::unleak(process, range.clone(), initial_perms);
            let res = mapping.update_permissions(perms);
            mapping.leak(); // be sure to not drop the mapping, even on error, else we we have troubes to show the panic
            res.expect("Could not setup memory protection");
        }

        debug!(
            "{}: 0x{:016X} -> 0x{:016X} (size=0x{:X})",
            name,
            range.start,
            range.end,
            range.len()
        );
    }
}

#[allow(dead_code)]
fn dump_processes_threads() {
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

#[allow(dead_code)]
fn do_ipc() {
    // create thread, send data and wait back

    let (echo_reader, main_sender) = kobject::Port::create(None).expect("failed to create ipc");
    let (main_reader, echo_sender) = kobject::Port::create(None).expect("failed to create ipc");

    let echo = move || {
        let mut message = echo_reader.blocking_receive().expect("receive failed");
        echo_sender.send(&mut message).expect("send failed");
    };

    let mut options = ThreadOptions::default();
    options.name("echo");

    kobject::Thread::start(echo, options).expect("could not create echo thread");

    let mut msg = unsafe { kobject::Message::new::<i32>(&42, &mut []) };
    main_sender.send(&mut msg).expect("send failed");

    let msg = main_reader.blocking_receive().expect("wait failed");

    assert!(unsafe { *msg.data::<i32>() } == 42);
    debug!("IPC ALL GOOD");
}

#[allow(dead_code)]
fn listen_threads() {
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

#[allow(dead_code)]
fn kmem_stats() {
    let stats = kobject::Memory::stats();
    const MEGA: usize = 1 * 1024 * 1024;
    debug!("Kernel memory allocator stats:");

    debug!(
        "phys: total={} ({}MB), free={} ({}MB)",
        stats.phys.total,
        stats.phys.total / MEGA,
        stats.phys.free,
        stats.phys.free / MEGA
    );
    debug!(
        "kvm: total={} ({:#X}), used={} ({:#X})",
        stats.kvm.total, stats.kvm.total, stats.kvm.used, stats.kvm.used
    );
    debug!(
        "kalloc: slabs: user={} ({}MB), allocated={} ({}MB)",
        stats.kalloc.slabs_user,
        stats.kalloc.slabs_user / MEGA,
        stats.kalloc.slabs_allocated,
        stats.kalloc.slabs_allocated / MEGA
    );
    debug!(
        "kalloc: kvm: user={} ({}MB), allocated={} ({}MB)",
        stats.kalloc.kvm_user,
        stats.kalloc.kvm_user / MEGA,
        stats.kalloc.kvm_allocated,
        stats.kalloc.kvm_allocated / MEGA
    );
}
