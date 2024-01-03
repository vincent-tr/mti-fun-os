#![no_std]
#![no_main]
#![allow(dead_code)]
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
#![feature(linked_list_cursors)]
#![feature(trait_alias)]
#![feature(async_closure)]
#![feature(noop_waker)]
#![feature(never_type)]

extern crate alloc;
extern crate bootloader_api;
extern crate lazy_static;

mod devices;
mod gdt;
mod interrupts;
mod logging;
mod memory;

mod user;

use crate::memory::VirtAddr;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use log::{error, info};
use syscalls::SyscallNumber;
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

    devices::init();
    interrupts::init_userland();
    user::init();

    interrupts::syscall_switch(SyscallNumber::InitSetup as usize);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();

    error!("PANIC: {info}");
    halt()
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
