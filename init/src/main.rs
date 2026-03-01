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

use core::{mem, slice};

use alloc::{boxed::Box, format};
use libruntime::{file, kobject, process, state, time};
use log::{debug, info};

fn main(info: &syscalls::init::InitInfo) {
    debug!("Init info: {:?}", info);

    idle::create_idle_process().expect("Could not create idle process");

    // tests::thread::dump_processes_threads();
    // tests::thread::listen_threads();
    // tests::thread::create_suspended();
    // tests::ipc::do_ipc();
    // tests::basic::kmem_stats();
    // tests::basic::test_unwind();
    // tests::thread::interval_second();
    // tests::sync::test_futex();
    // tests::sync::test_mutex();
    // tests::sync::test_rwlock();
    // tests::sync::test_async_mutex();
    // tests::sync::test_async_rwlock();

    start_base_servers(&info);
    setup_initial_filesystem(&info);

    // tests::process::list_processes();
    // tests::file::test_vfs();

    start_extended_servers(&info);

    unsafe { clean_init_info(&info) };
    // We cannot use info from here, so override it to make sure we don't accidentally use it
    let info = ();
    let _ = info;

    // init cannot exit, it runs the state server
    sleep_forever();
    // libruntime::exit();
}

fn start_base_servers(info: &syscalls::init::InitInfo) {
    let archive = unsafe {
        slice::from_raw_parts(
            info.archive_mapping.address as *const u8,
            info.archive_mapping.size,
        )
    };

    state_server::start();
    wait_port(state::iface::PORT_NAME);

    let init_binary = archive_find(archive, "init");
    let process_server_binary = archive_find(archive, "servers/core/process-server");
    loader::load("process-server", process_server_binary).expect("Could not load process server");
    wait_port(process::iface::PORT_NAME);

    unsafe {
        // Initialize process server
        process::initialize_process_server(init_binary, process_server_binary);

        // Initialize our own process stuff
        process::init();
    }

    // Required by memfs-server to be able to set timestamps to created directories and files
    let options = process::ProcessOptions::from_buffer(
        "time-server",
        archive_find(archive, "servers/core/time-server"),
    );
    let process = process::Process::spawn(options).expect("Could not spawn time server");

    let _ = process;
    wait_port(time::iface::PORT_NAME);

    info!("Current time: {}", time::get_wall_time());

    let options = process::ProcessOptions::from_buffer(
        "vfs-server",
        archive_find(archive, "servers/core/vfs-server"),
    );
    let process = process::Process::spawn(options).expect("Could not spawn vfs server");

    let _ = process;
    wait_port(file::vfs::iface::PORT_NAME);

    let options = process::ProcessOptions::from_buffer(
        "memfs-server",
        archive_find(archive, "servers/fs/memfs-server"),
    );
    let process = process::Process::spawn(options).expect("Could not spawn memfs server");

    let _ = process;
    wait_port("memfs-server");

    let options = process::ProcessOptions::from_buffer(
        "archivefs-server",
        archive_find(archive, "servers/fs/archivefs-server"),
    );
    let process = process::Process::spawn(options).expect("Could not spawn archivefs server");

    let _ = process;
    wait_port("archivefs-server");
}

fn setup_initial_filesystem(info: &syscalls::init::InitInfo) {
    debug!("Setting up initial filesystem...");

    let args = Box::new([0u8; 0]);

    file::mount("/", "memfs-server", args.as_slice()).expect("Could not mount memfs");

    file::Directory::create(
        "/mnt",
        file::Permissions::READ | file::Permissions::WRITE | file::Permissions::EXECUTE,
    )
    .expect("Could not create /mnt directory");

    file::Directory::create(
        "/mnt/archive",
        file::Permissions::READ | file::Permissions::WRITE | file::Permissions::EXECUTE,
    )
    .expect("Could not create /mnt/archive directory");

    let args = unsafe {
        slice::from_raw_parts(
            info.archive_mapping.address as *const u8,
            info.archive_mapping.size,
        )
    };

    file::mount("/mnt/archive", "archivefs-server", args).expect("Could not mount archivefs");

    debug!("Initial filesystem setup complete");

    let mounts = file::list_mounts().expect("Could not list mounts");
    for mount in mounts {
        info!(
            "Mounted '{}' at '{}'",
            mount.fs_port_name, mount.mount_point
        );
    }
}

fn start_extended_servers(info: &syscalls::init::InitInfo) {
    let options = process::ProcessOptions::from_path("/mnt/archive/servers/bus/pci-server")
        .expect("Failed to load pci-server");
    let process = process::Process::spawn(options).expect("Could not spawn pci server");
    let _ = process;

    let mut options =
        process::ProcessOptions::from_path("/mnt/archive/servers/core/display-server")
            .expect("Failed to load file");
    options.set_arg(
        "framebuffer.address",
        format!("{}", info.framebuffer.address),
    );
    options.set_arg(
        "framebuffer.byte_len",
        format!("{}", info.framebuffer.byte_len),
    );
    options.set_arg("framebuffer.width", format!("{}", info.framebuffer.width));
    options.set_arg("framebuffer.height", format!("{}", info.framebuffer.height));
    options.set_arg(
        "framebuffer.pixel_format.red_mask",
        format!("{}", info.framebuffer.pixel_format.red_mask),
    );
    options.set_arg(
        "framebuffer.pixel_format.green_mask",
        format!("{}", info.framebuffer.pixel_format.green_mask),
    );
    options.set_arg(
        "framebuffer.pixel_format.blue_mask",
        format!("{}", info.framebuffer.pixel_format.blue_mask),
    );
    options.set_arg(
        "framebuffer.bytes_per_pixel",
        format!("{}", info.framebuffer.bytes_per_pixel),
    );
    options.set_arg("framebuffer.stride", format!("{}", info.framebuffer.stride));

    let process = process::Process::spawn(options).expect("Could not spawn display server");

    let _ = process;
    //wait_port(display::iface::PORT_NAME);
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

/// Clean up the init info, unmapping the archive and such, as we don't need it anymore
///
/// # Safety
/// - This should only be called once, and after this, the info should not be used anymore
unsafe fn clean_init_info(info: &syscalls::init::InitInfo) {
    let process = kobject::Process::current();

    let clean_mapping = |mapping: &syscalls::init::Mapping| {
        let range = mapping.address..mapping.address + mapping.size;
        unsafe {
            let mapping = kobject::Mapping::unleak(process, range, kobject::Permissions::READ);
            // explicit
            mem::drop(mapping);
        }
    };

    clean_mapping(&info.archive_mapping);
    // Clean last because we cannot use info itself after this
    clean_mapping(&info.info_mapping);
}

fn sleep_forever() -> ! {
    debug!("Going to sleep...");
    loop {
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}
