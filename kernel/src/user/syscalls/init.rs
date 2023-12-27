use core::mem;

use crate::memory::{drop_initial_kernel_stack, page_aligned_up, Permissions, PAGE_SIZE};
use crate::user;
use crate::user::process::{self, Process};
use crate::user::syscalls::engine::unregister_syscall;
use crate::{memory::VirtAddr, user::MemoryObject};
use alloc::sync::Arc;
use log::info;
use syscalls::{Error, SyscallNumber, ThreadPriority};

const BASE_ADDRESS: VirtAddr = VirtAddr::new_truncate(0x200000);
const SIZE_OF_HEADERS: usize = PAGE_SIZE;

// https://docs.rs/include_bytes_aligned/latest/src/include_bytes_aligned/lib.rs.html#1-37
macro_rules! include_bytes_aligned {
    ($align_to:expr, $path:expr) => {{
        #[repr(C, align($align_to))]
        struct __Aligned<T: ?Sized>(T);

        static __DATA: &'static __Aligned<[u8]> = &__Aligned(*include_bytes!($path));

        &__DATA.0
    }};
}

pub fn setup(
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    // Unregister current syscall
    unregister_syscall(SyscallNumber::InitSetup);

    // Drop initial kernel stack (not used anymore, we are on regular interrupt stack)
    drop_initial_kernel_stack();

    info!("Loading init binary");
    let process = load();
    create_thread(process);

    user::thread::initial_setup_thread();

    Ok(())
}

fn load() -> Arc<Process> {
    // TODO: make path less static
    let binary = include_bytes_aligned!(8, "../../../../target/x86_64-mti_fun_os/debug/init");

    // Load init binary at fixed address
    let mem_size = page_aligned_up(binary.len());
    let memory_object = MemoryObject::new(mem_size).expect("Failed to create memory object");

    let process = process::create().expect("Failed to create init process");

    process
        .mmap(
            BASE_ADDRESS,
            mem_size,
            Permissions::READ | Permissions::WRITE | Permissions::EXECUTE,
            Some(memory_object),
            0,
        )
        .expect("Failed to map in init process");

    let mut access = process
        .vm_access(
            BASE_ADDRESS..BASE_ADDRESS + binary.len(),
            Permissions::READ | Permissions::WRITE,
        )
        .expect("Failed to access mapping");

    let dest = access.get_slice_mut::<u8>();
    dest.copy_from_slice(binary);

    mem::drop(access);

    process
}

fn create_thread(process: Arc<Process>) {
    // .text section is right after headers.
    // entry point is laid out at the begining of .text section
    let entry_point = BASE_ADDRESS + SIZE_OF_HEADERS;

    // Init does setup its stack itself.
    let stack_top = VirtAddr::zero();

    user::thread::create(
        process.clone(),
        ThreadPriority::Normal,
        entry_point,
        stack_top,
    );
}
