#![no_std]
#![no_main]

use libruntime::kobject;
use log::info;

extern crate alloc;
extern crate libruntime;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    info!("Display server started");

    for (name, value) in libruntime::process::SelfProcess::get().args_all() {
        info!("Arg: {} = {}", name, value);
    }

    let phys_addr = read_usize_arg("framebuffer.address");
    let size = read_usize_arg("framebuffer.byte_len");

    let memory_object =
        unsafe { libruntime::kobject::MemoryObject::open_iomem(phys_addr, size, false, true) }
            .expect("Failed to open framebuffer memory object");

    let proc = kobject::Process::current();
    let mapping = proc
        .map_mem(
            None,
            memory_object
                .size()
                .expect("Failed to get memory object size"),
            libruntime::kobject::Permissions::WRITE,
            &memory_object,
            0,
        )
        .expect("Failed to map framebuffer");
    let framebuffer = unsafe { mapping.as_buffer_mut() }.expect("Failed to obtain framebuffer");

    loop {
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}

fn read_usize_arg(name: &str) -> usize {
    let value = libruntime::process::SelfProcess::get()
        .arg(name)
        .expect("Failed to read argument");
    value
        .parse::<usize>()
        .expect("Failed to parse argument as usize")
}
