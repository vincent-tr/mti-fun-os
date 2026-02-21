use core::cmp::min;
use core::ops::Range;
use core::slice;

use crate::interrupts::SyscallArgs;
use crate::memory::{
    self, PAGE_SIZE, Permissions, drop_initial_kernel_stack, drop_initial_ramdisk, is_page_aligned,
    page_aligned_up,
};
use crate::user;
use crate::user::process;
use crate::user::syscalls::engine::unregister_syscall;
use crate::{memory::VirtAddr, user::MemoryObject};
use alloc::sync::Arc;
use log::info;
use syscalls::{SyscallNumber, ThreadPriority};

const BASE_ADDRESS: VirtAddr = VirtAddr::new_truncate(0x200000);
const SIZE_OF_HEADERS: usize = PAGE_SIZE;

pub fn setup(context: SyscallArgs) {
    let ramdisk = context.arg1()..context.arg2();

    // Unregister current syscall
    unregister_syscall(SyscallNumber::InitSetup);

    // Drop initial kernel stack (not used anymore, we are on regular interrupt stack)
    drop_initial_kernel_stack();

    info!("Loading init binary");
    let mobj = load_mem(&ramdisk);

    // Drop it before we create the process
    drop_initial_ramdisk();

    create_process(mobj, &ramdisk);

    user::thread::initial_setup_thread();
}

fn load_mem(ramdisk: &Range<usize>) -> Arc<MemoryObject> {
    // Load init binary memory contained in ramdisk
    let mem_size = page_aligned_up(ramdisk.len());
    let mobj = MemoryObject::new(mem_size).expect("Failed to create memory object");

    // Copy page by page
    assert!(is_page_aligned(ramdisk.start));

    for (index, frame) in mobj.frames_iter().enumerate() {
        let dest = unsafe { memory::access_phys(frame) };

        let source_start = ramdisk.start + index * PAGE_SIZE;
        let source_end = min(ramdisk.end, source_start + PAGE_SIZE);
        let size = source_end - source_start;
        let source = unsafe { slice::from_raw_parts(source_start as *const u8, size) };

        dest[0..size].copy_from_slice(source);
    }

    mobj
}

fn create_process(mobj: Arc<MemoryObject>, ramdisk: &Range<usize>) {
    let process = process::create("init").expect("Failed to create init process");

    process
        .mmap(
            BASE_ADDRESS,
            mobj.size(),
            Permissions::READ | Permissions::WRITE | Permissions::EXECUTE,
            Some(mobj),
            0,
        )
        .expect("Failed to map in init process");

    // .text section is right after headers.
    // entry point is laid out at the begining of .text section
    let entry_point = BASE_ADDRESS + SIZE_OF_HEADERS as u64;

    // Pass the binary size as argument (useful to load debug symbols)
    let arg: usize = ramdisk.len();

    // Init does setup its stack itself.
    let stack_top = VirtAddr::zero();

    user::thread::create(
        Some("entry"),
        process.clone(),
        false,
        ThreadPriority::Normal,
        entry_point,
        stack_top,
        arg,
        VirtAddr::zero(),
    );
}
