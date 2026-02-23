#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;

mod archive;
mod entry;
mod idle;
mod loader;
mod offsets;
mod state_server;
mod tests;

use alloc::boxed::Box;
use libruntime::{ipc, kobject, process, state, vfs};
use log::{debug, info};

fn main(info: Box<syscalls::init::InitInfo>) {
    debug!("Init info: {:#?}", info);

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
    // tests::vfs::test_vfs();

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
        "time-server",
        ipc::Buffer::new_local(archive::TIME_SERVER),
        &[],
        &[],
    )
    .expect("Could not spawn time server");

    let _ = process;
    sleep_forever(); ////////
    //wait_port(vfs::iface::PORT_NAME);

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
