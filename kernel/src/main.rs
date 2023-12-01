#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_sub_ptr)]
#![feature(slice_ptr_get)]
#![feature(const_slice_from_raw_parts_mut)]
#![feature(is_sorted)]
#![feature(slice_ptr_len)]
#![feature(allocator_api)]

extern crate bootloader_api;
extern crate lazy_static;
extern crate alloc;

mod gdt;
mod interrupts;
mod logging;
mod memory;

use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use log::{error, info};
use x86_64::VirtAddr;

use crate::memory::PAGE_SIZE;

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

    let free_before = memory::phys::stats().free;

    let mut frame = memory::phys::allocate().unwrap();
    let addr1 = memory::VMALLOC_START;
    let addr2 = memory::VMALLOC_START + PAGE_SIZE * 3;

    unsafe {
        let mut frame = frame.clone();

        memory::paging::KERNEL_ADDRESS_SPACE
            .map(
                addr1,
                &mut frame,
                memory::paging::Permissions::READ | memory::paging::Permissions::WRITE,
            )
            .unwrap();

        let data: *mut i64 = addr1.as_mut_ptr();
        *data = 42;

        memory::paging::KERNEL_ADDRESS_SPACE.unmap(addr1).unwrap();
    }

    // map frame somewhere else
    unsafe {
        let mut frame = frame.clone();

        memory::paging::KERNEL_ADDRESS_SPACE
            .map(
                addr2,
                &mut frame,
                memory::paging::Permissions::READ | memory::paging::Permissions::WRITE,
            )
            .unwrap();

            let data: *mut i64 = addr2.as_mut_ptr();

            info!("data={}", *data);

        memory::paging::KERNEL_ADDRESS_SPACE.unmap(addr2).unwrap();
    }

    core::mem::drop(frame);

    let free_after = memory::phys::stats().free;

    info!("free: before={free_before}, after={free_after}, eq={}", free_before == free_after);

    // Note:
    // boot_info is unmapped from here.
    // Do not used it.

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
