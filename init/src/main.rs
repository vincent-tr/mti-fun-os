#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;

mod entry;
mod idle;
mod loader;
mod offsets;
mod state_server;
mod tests;

use core::slice;

use alloc::{boxed::Box, format};
use libruntime::{ipc, kobject, process, state, time, vfs};
use log::{debug, info};

fn main(info: &syscalls::init::InitInfo) {
    debug!("Init info: {:?}", info);

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

    start_servers(&info);
    setup_initial_filesystem();

    // tests::process::list_processes();
    // tests::vfs::test_vfs();

    // init cannot exit, it runs the state server
    sleep_forever();
    // libruntime::exit();
}

fn start_servers(info: &syscalls::init::InitInfo) {
    let archive = unsafe {
        slice::from_raw_parts(
            info.archive_mapping.address as *const u8,
            info.archive_mapping.size,
        )
    };

    state_server::start();
    wait_port(state::iface::PORT_NAME);

    let init_binary = archive_find(archive, "init");
    let process_server_binary = archive_find(archive, "servers/process-server");
    loader::load("process-server", process_server_binary).expect("Could not load process server");
    wait_port(process::iface::PORT_NAME);

    // Initialize process server
    unsafe { process::initialize_process_server(init_binary, process_server_binary) };
    // Initialize our own process stuff
    process::SelfProcess::get();

    let process = process::Process::spawn(
        "time-server",
        ipc::Buffer::new_local(archive_find(archive, "servers/time-server")),
        &[],
        &[],
    )
    .expect("Could not spawn time server");

    let _ = process;
    wait_port(time::iface::PORT_NAME);

    info!("Current time: {}", time::get_wall_time());

    // TODO: move in archive
    let process = process::Process::spawn(
        "display-server",
        ipc::Buffer::new_local(archive_find(archive, "servers/display-server")),
        &[],
        &[
            (
                "framebuffer.address",
                &format!("{}", info.framebuffer.address),
            ),
            (
                "framebuffer.byte_len",
                &format!("{}", info.framebuffer.byte_len),
            ),
            ("framebuffer.width", &format!("{}", info.framebuffer.width)),
            (
                "framebuffer.height",
                &format!("{}", info.framebuffer.height),
            ),
            (
                "framebuffer.pixel_format.red_mask",
                &format!("{}", info.framebuffer.pixel_format.red_mask),
            ),
            (
                "framebuffer.pixel_format.green_mask",
                &format!("{}", info.framebuffer.pixel_format.green_mask),
            ),
            (
                "framebuffer.pixel_format.blue_mask",
                &format!("{}", info.framebuffer.pixel_format.blue_mask),
            ),
            (
                "framebuffer.bytes_per_pixel",
                &format!("{}", info.framebuffer.bytes_per_pixel),
            ),
            (
                "framebuffer.stride",
                &format!("{}", info.framebuffer.stride),
            ),
        ],
    )
    .expect("Could not spawn display server");

    let _ = process;
    //wait_port(display::iface::PORT_NAME);

    let process = process::Process::spawn(
        "vfs-server",
        ipc::Buffer::new_local(archive_find(archive, "servers/vfs-server")),
        &[],
        &[],
    )
    .expect("Could not spawn vfs server");

    let _ = process;
    wait_port(vfs::iface::PORT_NAME);

    let process = process::Process::spawn(
        "memfs-server",
        ipc::Buffer::new_local(archive_find(archive, "servers/memfs-server")),
        &[],
        &[],
    )
    .expect("Could not spawn memfs server");

    let _ = process;
    wait_port("memfs-server");

    let process = process::Process::spawn(
        "archivefs-server",
        ipc::Buffer::new_local(archive_find(archive, "servers/archivefs-server")),
        &[],
        &[],
    )
    .expect("Could not spawn archivefs server");

    let _ = process;
    //wait_port("archivefs-server");
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

/// Find a file in the archive, and return its content as a byte slice.
fn archive_find<'a>(archive: &'a [u8], name: &str) -> &'a [u8] {
    for entry in cpio_reader::iter_files(archive) {
        if entry.name() == name {
            return entry.file();
        }
    }

    panic!("Could not find '{}' in archive", name);
}

/// Wait for a server port to be available
fn wait_port(name: &'static str) {
    loop {
        match kobject::Port::open_by_name(name) {
            Ok(_) => break,
            Err(kobject::Error::ObjectNotFound) => {
                libruntime::time::sleep(libruntime::time::Duration::milliseconds(100));
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
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}
