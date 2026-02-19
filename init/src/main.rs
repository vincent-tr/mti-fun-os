#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(used_with_arg)]

extern crate alloc;

mod archive;
mod idle;
mod loader;
mod offsets;
mod state_server;
mod tests;

use core::{arch::naked_asm, hint::unreachable_unchecked, ops::Range};

use alloc::boxed::Box;
use libruntime::{
    ipc,
    kobject::{self, Permissions, ThreadOptions, PAGE_SIZE},
    process, state, vfs,
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
    // let binary = unsafe { slice::from_raw_parts(offsets::global().start as *const u8, binary_len) };
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

    // tests::thread::dump_processes_threads();
    // tests::thread::listen_threads();
    // tests::ipc::do_ipc();
    // tests::basic::kmem_stats();
    // tests::basic::test_unwind();
    // tests::thread::interval_second();
    // tests::sync::test_futex();
    // tests::sync::test_mutex();
    // tests::sync::test_rwlock();
    // tests::sync::test_async_mutex();
    // tests::sync::test_async_rwlock();

    start_servers();
    setup_initial_filesystem();

    // tests::process::list_processes();
    tests::vfs::test_vfs();

    // init cannot exit, it runs the state server
    sleep_forever();
    // libruntime::exit();
}

fn start_servers() {
    state_server::start();
    wait_port(state::iface::PORT_NAME);

    loader::load("process-server", archive::PROCESS_SERVER).expect("Could not load process server");
    wait_port(process::iface::PORT_NAME);

    let process = process::Process::spawn(
        "vfs-server",
        ipc::Buffer::new_local(archive::VFS_SERVER),
        &[],
        &[],
    )
    .expect("Could not spawn vfs server");

    let _ = process;
    wait_port(vfs::iface::PORT_NAME);

    let process = process::Process::spawn(
        "memfs-server",
        ipc::Buffer::new_local(archive::MEMFS_SERVER),
        &[],
        &[],
    )
    .expect("Could not spawn memfs server");

    let _ = process;
    wait_port("memfs-server");
}

fn setup_initial_filesystem() {
    debug!("Setting up initial filesystem...");

    let args = Box::new([0u8; 0]);

    vfs::mount("/", "memfs-server", args.as_slice()).expect("Could not mount memfs");
    vfs::Directory::create(
        "/init",
        vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE,
    )
    .expect("Could not create /init directory");

    debug!("Initial filesystem setup complete");

    let mounts = vfs::list_mounts().expect("Could not list mounts");
    for mount in mounts {
        info!(
            "Mounted '{}' at '{}'",
            mount.fs_port_name, mount.mount_point
        );
    }
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

/// Wait for a server port to be available
fn wait_port(name: &'static str) {
    loop {
        match kobject::Port::open_by_name(name) {
            Ok(_) => break,
            Err(kobject::Error::ObjectNotFound) => {
                libruntime::timer::sleep(libruntime::timer::Duration::from_milliseconds(100));
                debug!("waiting for '{}' port...", name);
            }
            Err(e) => panic!("Could not open '{}' port: {}", name, e),
        }
    }

    info!("found '{}' port", name);
}

fn sleep_forever() -> ! {
    debug!("Going to sleep...");
    loop {
        libruntime::timer::sleep(libruntime::timer::Duration::from_seconds(1));
    }
}
