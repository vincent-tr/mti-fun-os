#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

mod logging;
mod offsets;

use core::{arch::asm, hint::unreachable_unchecked, panic::PanicInfo};

use bit_field::BitArray;
use libsyscalls::{
    ipc, process, thread, Exception, Handle, Permissions, SyscallResult, ThreadContextRegister,
    ThreadEventType, ThreadPriority,
};
use log::{debug, error, info};

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

const PAGE_SIZE: usize = 4096;

extern "C" fn main() -> ! {
    logging::init();

    let self_proc = libsyscalls::process::open_self().expect("Could not open self process");

    apply_memory_protections(&self_proc);

    // small stack, does not do much
    let idle_stack =
        libsyscalls::memory_object::create(PAGE_SIZE).expect("Could not create idle task stack");

    let stack_addr = libsyscalls::process::mmap(
        &self_proc,
        None,
        PAGE_SIZE,
        Permissions::READ | Permissions::WRITE,
        Some(&idle_stack),
        0,
    )
    .expect("Could not map idle task stack");
    let stack_top = stack_addr + PAGE_SIZE;

    libsyscalls::thread::create(&self_proc, ThreadPriority::Idle, idle, stack_top, 0, 0)
        .expect("Could create idle task");

    dump_processes_threads();

    listen_threads(&self_proc);

    do_ipc(&self_proc);

    //debug!("Exiting");
    //process::exit().expect("Could not exit process");
    thread::exit().expect("Could not exit thread");
    unsafe { unreachable_unchecked() };
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

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info}");

    // Note: if case we failed exit, we cannot do much more.
    let _ = process::exit();
    unsafe { unreachable_unchecked() };
}

extern "C" fn idle(_arg: usize) -> ! {
    // TODO: better sleep
    loop {}
}

fn do_ipc(self_proc: &Handle) {
    // create thread, send data and wait back

    let (reader1, sender1) = libsyscalls::ipc::create(Some("chan1")).expect("failed to create ipc");
    let (reader2, sender2) = libsyscalls::ipc::create(Some("chan2")).expect("failed to create ipc");

    unsafe {
        EXC.reader = reader1;
        EXC.sender = sender2;
    }

    create_thread(self_proc, echo);

    let msg = libsyscalls::Message {
        data: [42; libsyscalls::Message::DATA_SIZE],
        handles: [unsafe { core::mem::transmute(Handle::invalid()) };
            libsyscalls::Message::HANDLE_COUNT],
    };

    libsyscalls::ipc::send(&sender1, &msg).expect("send failed");

    wait_one(&reader2).expect("wait failed");
    let msg = libsyscalls::ipc::receive(&reader2).expect("receive failed");

    assert!(msg.data[0] == 42);
    debug!("IPC ALL GOOD");
}

struct Exchange {
    reader: Handle,
    sender: Handle,
}

static mut EXC: Exchange = Exchange {
    reader: Handle::invalid(),
    sender: Handle::invalid(),
};

extern "C" fn echo(_arg: usize) -> ! {
    // take from EXC
    let mut reader = Handle::invalid();
    let mut sender = Handle::invalid();
    unsafe {
        core::mem::swap(&mut reader, &mut EXC.reader);
        core::mem::swap(&mut sender, &mut EXC.sender);
    }

    wait_one(&reader).expect("wait failed");
    let msg = libsyscalls::ipc::receive(&reader).expect("receive failed");

    libsyscalls::ipc::send(&sender, &msg).expect("send failed");

    thread::exit().expect("Could not exit thread");
    unsafe { unreachable_unchecked() };
}

fn listen_threads(self_proc: &Handle) {
    create_thread(self_proc, do_listen_threads);
}

extern "C" fn do_listen_threads(_arg: usize) -> ! {
    let (reader, sender) =
        libsyscalls::ipc::create(Some("thread-listener")).expect("failed to create ipc");

    let _listener = libsyscalls::listener::create_thread(&sender, None)
        .expect("failed to create thread listener");

    let (mut thread_debugbreak, mut thread_pagefault) = {
        let self_proc = libsyscalls::process::open_self().expect("Could not open self process");
        (
            create_thread(&self_proc, debugbreak),
            create_thread(&self_proc, page_fault),
        )
    };

    let thread_debugbreak_info =
        libsyscalls::thread::info(&thread_debugbreak).expect("Could not get thread info");
    let thread_pagefault_info =
        libsyscalls::thread::info(&thread_pagefault).expect("Could not get thread info");

    loop {
        wait_one(&reader).expect("wait failed");
        let msg = libsyscalls::ipc::receive(&reader).expect("receive failed");

        let ptr = msg.data.as_ptr() as *const libsyscalls::ThreadEvent;
        let event = unsafe { &*ptr };

        debug!("Thread event: {:?}", event);

        if let ThreadEventType::Error = event.r#type {
            let thread = if event.tid == thread_debugbreak_info.tid {
                let mut thread = Handle::invalid();
                core::mem::swap(&mut thread, &mut thread_debugbreak);
                thread
            } else if event.tid == thread_pagefault_info.tid {
                let mut thread = Handle::invalid();
                core::mem::swap(&mut thread, &mut thread_pagefault);
                thread
            } else {
                panic!("unexpected error");
            };

            let err =
                libsyscalls::thread::error_info(&thread).expect("could not get thread error info");

            debug!("Thread error: {:?}", err);

            match err {
                Exception::Breakpoint => {
                    // change context: update rax
                    let context =
                        libsyscalls::thread::context(&thread).expect("get context failed");
                    debug!("Thread RAX = {}", context.rax);
                    libsyscalls::thread::update_context(
                        &thread,
                        &[(ThreadContextRegister::Rax, context.rax + 1)],
                    )
                    .expect("set context failed");

                    debug!("Thread resume");
                    libsyscalls::thread::resume(&thread).expect("resume failed");
                }
                Exception::PageFault(_error_code, address) => {
                    let self_proc =
                        libsyscalls::process::open_self().expect("Could not open self process");
                    let page = libsyscalls::memory_object::create(PAGE_SIZE)
                        .expect("Could not create page");

                    libsyscalls::process::mmap(
                        &self_proc,
                        Some(address),
                        PAGE_SIZE,
                        Permissions::READ | Permissions::WRITE,
                        Some(&page),
                        0,
                    )
                    .expect("Could not map page");

                    debug!("Thread resume");
                    libsyscalls::thread::resume(&thread).expect("resume failed");
                }
                _ => {}
            }

            // thread handle will be dropped here
        }
    }
}

extern "C" fn debugbreak(arg: usize) -> ! {
    debug!("debugbreak: arg={arg}");
    unsafe {
        set_tls(42);
    }

    let mut value = 42;
    unsafe {
        asm!("int3", inlateout("rax") value => value, options(nostack, preserves_flags));
    }

    debug!("debugbreak: resumed (value={value})");

    debug!("debugbreak: tls={}", unsafe { get_tls() });

    thread::exit().expect("Could not exit thread");
    unsafe { unreachable_unchecked() };
}

const PAGE_FAULT_ADDR: usize = 0x10000;

extern "C" fn page_fault(arg: usize) -> ! {
    debug!("page_fault: arg={arg}");
    unsafe {
        set_tls(43);
    }

    let ptr = PAGE_FAULT_ADDR as *mut u8;
    unsafe { *ptr = 42 };

    debug!("page_fault: resumed");

    debug!("page_fault: tls={}", unsafe { get_tls() });

    thread::exit().expect("Could not exit thread");
    unsafe { unreachable_unchecked() };
}

// Helpers

fn create_thread(self_proc: &Handle, entry_point: extern "C" fn(usize) -> !) -> Handle {
    // small stack, does not do much
    let thread_stack =
        libsyscalls::memory_object::create(PAGE_SIZE).expect("Could not create thread task stack");

    let stack_addr = libsyscalls::process::mmap(
        &self_proc,
        None,
        PAGE_SIZE,
        Permissions::READ | Permissions::WRITE,
        Some(&thread_stack),
        0,
    )
    .expect("Could not map thread task stack");
    let stack_top = stack_addr + PAGE_SIZE;

    // small stack, does not do much
    let tls = libsyscalls::memory_object::create(PAGE_SIZE).expect("Could not create tls");

    let tls_addr = libsyscalls::process::mmap(
        &self_proc,
        None,
        PAGE_SIZE,
        Permissions::READ | Permissions::WRITE,
        Some(&tls),
        0,
    )
    .expect("Could not map tls");

    libsyscalls::thread::create(
        &self_proc,
        ThreadPriority::Normal,
        entry_point,
        stack_top,
        42,
        tls_addr,
    )
    .expect("Could create echo task")
}

fn wait_one(port: &Handle) -> SyscallResult<()> {
    let ports = &[unsafe { port.as_syscall_value() }];
    let ready = &mut [0u8];

    ipc::wait(ports, ready)?;

    assert!(ready.get_bit(0));

    Ok(())
}

unsafe fn set_tls(value: usize) {
    asm!("mov fs:[0], {value};", value = in(reg)value, options(nostack, preserves_flags));
}

unsafe fn get_tls() -> usize {
    let mut value: usize;
    asm!("mov {value}, fs:[0];", value = out(reg)value, options(nostack, preserves_flags));
    value
}
