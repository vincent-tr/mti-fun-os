#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_sub_ptr)]
#![feature(slice_ptr_get)]
#![feature(const_slice_from_raw_parts_mut)]
#![feature(is_sorted)]
#![feature(slice_ptr_len)]
#![feature(allocator_api)]
#![feature(const_mut_refs)]
#![feature(btree_cursors)]
#![feature(let_chains)]
#![feature(const_trait_impl)]
#![feature(naked_functions)]
#![feature(asm_const)]

extern crate alloc;
extern crate bootloader_api;
extern crate lazy_static;

mod gdt;
mod interrupts;
mod logging;
mod memory;

mod init;
mod user;

use crate::{interrupts::switch_to_userland, memory::VirtAddr};
use crate::{
    memory::{Permissions, PAGE_SIZE},
    user::MemoryObject,
};
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::mem;
use core::panic::PanicInfo;
use log::{error, info};
use x86_64::registers::model_specific::{Efer, EferFlags};

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.dynamic_range_start = Some(0xFFFF_8000_0000_0000);
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_8080_0000_0000));
    config
};

entry_point!(kernel_main, config = &CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    logging::init();

    let efer_flags = Efer::read();
    assert!(efer_flags.contains(EferFlags::LONG_MODE_ENABLE));
    assert!(efer_flags.contains(EferFlags::LONG_MODE_ACTIVE));

    let version = &boot_info.api_version;
    info!(
        "Starting kernel with boot info v{}.{}.{}",
        version.version_major(),
        version.version_minor(),
        version.version_patch()
    );

    let physical_memory_offset = VirtAddr::new(*boot_info.physical_memory_offset.as_ref().unwrap());

    gdt::init();
    interrupts::init_base();
    memory::init(physical_memory_offset, &boot_info.memory_regions);

    // Note:
    // boot_info is unmapped from here.
    // Do not used it.

    // From here we can use normal allocations in the kernel.

    interrupts::init_userland();
    user::init();

    let (process, entry_point) = init::load();

    const INIT_STACK_SIZE: usize = 5 * PAGE_SIZE;

    let user_stack_mobj =
        MemoryObject::new(INIT_STACK_SIZE).expect("Failed to allocate user stack");
    let user_stack = process
        .map(
            VirtAddr::zero(),
            user_stack_mobj.size(),
            Permissions::READ | Permissions::WRITE,
            Some(user_stack_mobj),
            0,
        )
        .expect("Failed to map user stack");
    let user_stack_top = user_stack + INIT_STACK_SIZE;

    unsafe {
        let as_ptr = process.address_space().as_mut_ptr();
        // Note: should keep ref on process since it's installed
        memory::set_current_address_space(&*as_ptr);
    }

    // TODO: manage current thread/process properly
    user::temp_set_process(process.clone());

    info!("init entry point = {entry_point:?}");
    info!("init stack = {user_stack:?} -> {user_stack_top:?}");

    mem::drop(process);

    // TODO: clean initial kernel stack
    switch_to_userland(entry_point, user_stack_top);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info}");
    halt()
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
