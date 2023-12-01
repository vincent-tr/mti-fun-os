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

extern crate bootloader_api;
extern crate lazy_static;

mod gdt;
mod interrupts;
mod logging;
mod memory;

use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use log::{error, info};
use x86_64::VirtAddr;

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.dynamic_range_start = Some(0xFFFF_8000_0000_0000);
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_8080_0000_0000));
    config
};

entry_point!(kernel_main, config = &CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    logging::init();

    let version = &boot_info.api_version;
    info!(
        "Starting kernel with boot info v{}.{}.{}",
        version.version_major(),
        version.version_minor(),
        version.version_patch()
    );

    let physical_memory_offset = VirtAddr::new(*boot_info.physical_memory_offset.as_ref().unwrap());

    gdt::init();
    interrupts::init_idt();
    memory::phys::init(physical_memory_offset, &boot_info.memory_regions);
    memory::paging::init(physical_memory_offset);

    // Note:
    // boot_info is unmapped from here.
    // Do not used it.

    memory::kvm::init();

    let free_before = memory::phys::stats().free;

    let addr1 = memory::kvm::allocate(1).unwrap();

    info!("Got address {addr1:?}");

    unsafe {
        let data: *mut i64 = addr1.as_mut_ptr();
        *data = 42;
    }

    memory::kvm::deallocate(addr1, 1);

    let free_after = memory::phys::stats().free;

    info!("free: before={free_before}, after={free_after}, eq={}", free_before == free_after);

    panic!("End of main!");
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

/*


/// Performs the actual context switch.
unsafe fn context_switch(addresses: Addresses) -> ! {
    unsafe {
        asm!(
            r#"
            xor rbp, rbp
            mov cr3, {}
            mov rsp, {}
            push 0
            jmp {}
            "#,
            in(reg) addresses.page_table.start_address().as_u64(),
            in(reg) addresses.stack_top.as_u64(),
            in(reg) addresses.entry_point.as_u64(),
            in("rdi") addresses.boot_info as *const _ as usize,
        );
    }
    unreachable!();
}

*/
