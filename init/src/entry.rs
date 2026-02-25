use core::{arch::naked_asm, hint::unreachable_unchecked, mem::size_of, ops::Range};

use crate::{main, offsets};
use alloc::boxed::Box;
use libruntime::{
    kobject::{self, PAGE_SIZE, Permissions, ThreadOptions},
    memory::align_up,
};
use log::debug;

// Special init start: need to setup its own stack
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text_entry")]
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

extern "C" fn entry(init_info_ptr: usize) -> ! {
    libruntime::init();

    let init_info = unsafe { &*(init_info_ptr as *const syscalls::init::InitInfo) };

    apply_memory_protections(init_info.init_mapping.size);

    let info = unsafe { load_init_info(init_info) };

    // Jump to a safer thread, with better stack
    let mut options = ThreadOptions::default();
    options.name("main");
    kobject::Thread::start(|| main(info), options).expect("Could not start main thread");

    libsyscalls::thread::exit().expect("Failed to exit thread");
    unsafe { unreachable_unchecked() };
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

/// Safety: do not use reference to init info after this, as the memory used by the kernel to pass it will be released.
unsafe fn load_init_info(init_info: &syscalls::init::InitInfo) -> Box<syscalls::init::InitInfo> {
    let data = Box::new(init_info.clone());

    // Release the memory used by the kernel to pass the init info, as we have copied it
    let addr = init_info as *const syscalls::init::InitInfo as usize;
    assert!(addr % PAGE_SIZE == 0);
    let size = align_up(size_of::<syscalls::init::InitInfo>(), PAGE_SIZE);

    unsafe {
        kobject::Mapping::unleak(
            kobject::Process::current(),
            addr..addr + size,
            Permissions::READ,
        )
    }
    .leak();

    data
}
