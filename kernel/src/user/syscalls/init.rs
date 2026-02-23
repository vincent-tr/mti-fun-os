use core::cmp::min;
use core::{mem, slice};

use crate::interrupts::SyscallArgs;
use crate::memory::{
    self, PAGE_SIZE, Permissions, drop_initial_kernel_stack, drop_initial_ramdisk, page_aligned_up,
};
use crate::user;
use crate::user::process;
use crate::user::syscalls::engine::unregister_syscall;
use crate::{memory::VirtAddr, user::MemoryObject};
use alloc::boxed::Box;
use alloc::sync::Arc;
use log::info;
use syscalls::{SyscallNumber, ThreadPriority, init::InitInfo};

const BASE_ADDRESS: VirtAddr = VirtAddr::new_truncate(0x200000);
const SIZE_OF_HEADERS: usize = PAGE_SIZE;

pub fn setup(context: SyscallArgs) {
    let ramdisk = context.arg1()..context.arg2();
    let init_info = unsafe { Box::from_raw(context.arg3() as *mut InitInfo) };

    // Unregister current syscall
    unregister_syscall(SyscallNumber::InitSetup);

    // Drop initial kernel stack (not used anymore, we are on regular interrupt stack)
    drop_initial_kernel_stack();

    info!("Loading init binary");
    let ramdisk_buffer =
        unsafe { slice::from_raw_parts(ramdisk.start as *const u8, ramdisk.end - ramdisk.start) };
    let init_binary_mobj = load_mem(ramdisk_buffer);

    // Drop it before we create the process
    drop_initial_ramdisk();

    let init_info_buffer = unsafe {
        slice::from_raw_parts(
            init_info.as_ref() as *const InitInfo as *const u8,
            mem::size_of::<InitInfo>(),
        )
    };
    let init_info_mobj = load_mem(init_info_buffer);

    // Drop the structure, we don't need it anymore (and we don't want to leak it to the process)
    mem::drop(init_info);

    create_process(init_binary_mobj, init_info_mobj);

    user::thread::initial_setup_thread();
}

fn load_mem(buffer: &[u8]) -> Arc<MemoryObject> {
    // Load init binary memory contained in ramdisk
    let mem_size = page_aligned_up(buffer.len());
    let mobj = MemoryObject::new(mem_size).expect("Failed to create memory object");

    // Copy page by page
    let buffer_start = buffer.as_ptr() as usize;
    let buffer_end = buffer_start + buffer.len();

    for (index, frame) in mobj.frames_iter().enumerate() {
        let dest = unsafe { memory::access_phys(frame) };

        let source_start = buffer_start + index * PAGE_SIZE;
        let source_end = min(buffer_end, source_start + PAGE_SIZE);
        let size = source_end - source_start;
        let source = unsafe { slice::from_raw_parts(source_start as *const u8, size) };

        dest[0..size].copy_from_slice(source);
    }

    mobj
}

fn create_process(init_binary_mobj: Arc<MemoryObject>, init_info_mobj: Arc<MemoryObject>) {
    let process = process::create("init").expect("Failed to create init process");

    process
        .mmap(
            BASE_ADDRESS,
            init_binary_mobj.size(),
            Permissions::READ | Permissions::WRITE | Permissions::EXECUTE,
            Some(init_binary_mobj),
            0,
        )
        .expect("Failed to map in init process");

    let init_info_ptr = process
        .mmap(
            VirtAddr::zero(),
            init_info_mobj.size(),
            Permissions::READ,
            Some(init_info_mobj),
            0,
        )
        .expect("Failed to map init info in init process");

    // .text section is right after headers.
    // entry point is laid out at the begining of .text section
    let entry_point = BASE_ADDRESS + SIZE_OF_HEADERS as u64;

    // Pass the init info pointer as argument
    let arg: usize = init_info_ptr.as_u64() as usize;

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
