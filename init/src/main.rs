#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;

mod entry;
mod idle;
mod info;
mod loader;
mod offsets;
mod state_server;
mod tests;

use core::mem;

use alloc::{boxed::Box, format};
use info::InitInfo;
use libruntime::{file, kobject, process, state, time};
use log::{debug, info};

fn main(info: InitInfo) {
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

    start_core_servers(&info);
    setup_initial_filesystem(&info);

    // tests::process::list_processes();
    // tests::file::test_vfs();

    start_extended_servers(&info);

    mem::drop(info);

    // init cannot exit, it runs the state server
    sleep_forever();
    // libruntime::exit();
}

fn start_core_servers(info: &InitInfo) {
    let helper = CoreServersSetupHelper::new(info);

    state_server::start();
    wait_port(state::iface::PORT_NAME);

    let init_binary = helper.archive_find("init");
    let process_server_binary = helper.archive_find("servers/core/process-server");
    loader::load("process-server", process_server_binary).expect("Could not load process server");
    wait_port(process::iface::PORT_NAME);

    unsafe {
        // Initialize process server
        process::initialize_process_server(init_binary, process_server_binary);

        // Initialize our own process stuff
        process::init();
    }

    // Required by memfs-server to be able to set timestamps to created directories and files
    helper.start_server("servers/core/time-server", time::iface::PORT_NAME);

    info!("Current time: {}", time::get_wall_time());

    helper.start_server("servers/core/vfs-server", file::vfs::iface::PORT_NAME);
    helper.start_server("servers/fs/memfs-server", "memfs-server");
    helper.start_server("servers/fs/archivefs-server", "archivefs-server");
}

fn setup_initial_filesystem(info: &InitInfo) {
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

    file::mount("/mnt/archive", "archivefs-server", info.archive_buffer())
        .expect("Could not mount archivefs");

    debug!("Initial filesystem setup complete");

    let mounts = file::list_mounts().expect("Could not list mounts");
    for mount in mounts {
        info!(
            "Mounted '{}' at '{}'",
            mount.fs_port_name, mount.mount_point
        );
    }
}

fn start_extended_servers(info: &InitInfo) {
    let options = process::ProcessOptions::from_path("/mnt/archive/servers/drivers/bus/pci-server")
        .expect("Failed to load pci-server");
    let process = process::Process::spawn(options).expect("Could not spawn pci server");
    let _ = process;

    let options =
        process::ProcessOptions::from_path("/mnt/archive/servers/drivers/test/edu/edu-server")
            .expect("Failed to load edu-server");
    let process = process::Process::spawn(options).expect("Could not spawn edu server");
    let _ = process;

    let mut options =
        process::ProcessOptions::from_path("/mnt/archive/servers/core/display-server")
            .expect("Failed to load file");
    options.set_arg(
        "framebuffer.address",
        format!("{}", info.framebuffer().address),
    );
    options.set_arg(
        "framebuffer.byte_len",
        format!("{}", info.framebuffer().byte_len),
    );
    options.set_arg("framebuffer.width", format!("{}", info.framebuffer().width));
    options.set_arg(
        "framebuffer.height",
        format!("{}", info.framebuffer().height),
    );
    options.set_arg(
        "framebuffer.pixel_format.red_mask",
        format!("{}", info.framebuffer().pixel_format.red_mask),
    );
    options.set_arg(
        "framebuffer.pixel_format.green_mask",
        format!("{}", info.framebuffer().pixel_format.green_mask),
    );
    options.set_arg(
        "framebuffer.pixel_format.blue_mask",
        format!("{}", info.framebuffer().pixel_format.blue_mask),
    );
    options.set_arg(
        "framebuffer.bytes_per_pixel",
        format!("{}", info.framebuffer().bytes_per_pixel),
    );
    options.set_arg(
        "framebuffer.stride",
        format!("{}", info.framebuffer().stride),
    );

    let process = process::Process::spawn(options).expect("Could not spawn display server");

    let _ = process;
    //wait_port(display::iface::PORT_NAME);
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

/// A helper struct to setup the core servers.
#[derive(Debug)]
struct CoreServersSetupHelper<'a> {
    archive: &'a [u8],
}

impl<'a> CoreServersSetupHelper<'a> {
    /// Create a new CoreServersSetupHelper from the init info, which contains the archive mapping.
    pub fn new(info: &'a InitInfo) -> Self {
        Self {
            archive: info.archive_buffer(),
        }
    }

    /// Start a server from the archive.
    pub fn start_server(&self, archive_path: &str, port_name: &'static str) {
        let name = archive_path
            .rsplit('/')
            .next()
            .expect("Could not get server name");
        let options = process::ProcessOptions::from_buffer(name, self.archive_find(archive_path));

        let process =
            process::Process::spawn(options).expect(&format!("Could not spawn {} server", name,));

        let _ = process;
        wait_port(port_name);
    }

    /// Find a file in the archive, and return its contents as a byte slice.
    pub fn archive_find(&self, path: &str) -> &'a [u8] {
        for entry in cpio_reader::iter_files(self.archive) {
            if entry.name() == path {
                return entry.file();
            }
        }

        panic!("Could not find '{}' in archive", path);
    }
}

fn sleep_forever() -> ! {
    debug!("Going to sleep...");
    loop {
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}
