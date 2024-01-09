#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

mod logging;
mod offsets;

use core::{arch::asm, hint::unreachable_unchecked, panic::PanicInfo};

use bit_field::BitArray;
use libsyscalls::{
    ipc, thread, Exception, Handle, Permissions, SyscallResult, ThreadEventType, ThreadPriority,
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

    libsyscalls::thread::create(&self_proc, ThreadPriority::Idle, idle, stack_top)
        .expect("Could create idle task");

    dump_processes_threads();

    listen_threads(&self_proc);

    do_ipc(&self_proc);

    let _thread1 = create_thread(&self_proc, debugbreak);
    let _thread2 = create_thread(&self_proc, page_fault);

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

    halt();
}

fn idle() -> ! {
    halt();
}

fn halt() -> ! {
    // TODO: better sleep
    loop {}
}

fn debugbreak() -> ! {
    unsafe {
        asm!("int3", options(nomem, nostack));
    }

    debug!("debugbreak: resumed");

    thread::exit().expect("Could not exit thread");
    unsafe { unreachable_unchecked() };
}

fn page_fault() -> ! {
    let ptr = 0x42 as *mut u8;
    unsafe { *ptr = 42 };

    debug!("page_fault: resumed");

    thread::exit().expect("Could not exit thread");
    unsafe { unreachable_unchecked() };
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

fn echo() -> ! {
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

fn do_listen_threads() -> ! {
    let (reader, sender) =
        libsyscalls::ipc::create(Some("thread-listener")).expect("failed to create ipc");

    let _listener = libsyscalls::listener::create_thread(&sender, None)
        .expect("failed to create thread listener");

    loop {
        wait_one(&reader).expect("wait failed");
        let msg = libsyscalls::ipc::receive(&reader).expect("receive failed");

        let ptr = msg.data.as_ptr() as *const libsyscalls::ThreadEvent;
        let event = unsafe { &*ptr };

        debug!("Thread event: {:?}", event);

        if let ThreadEventType::Error = event.r#type {
            let thread = libsyscalls::thread::open(event.tid).expect("could not open error thread");
            let err =
                libsyscalls::thread::error_info(&thread).expect("could not get thread error info");

            debug!("Thread error: {:?}", err);

            if let Exception::Breakpoint = err {
                debug!("Thread resume");
                libsyscalls::thread::resume(&thread).expect("resume failed");
            }
        }
    }
}

// Helpers

fn create_thread(self_proc: &Handle, entry_point: fn() -> !) -> Handle {
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

    libsyscalls::thread::create(&self_proc, ThreadPriority::Normal, entry_point, stack_top)
        .expect("Could create echo task")
}

fn wait_one(port: &Handle) -> SyscallResult<()> {
    let ports = &[unsafe { port.as_syscall_value() }];
    let ready = &mut [0u8];

    ipc::wait(ports, ready)?;

    assert!(ready.get_bit(0));

    Ok(())
}
