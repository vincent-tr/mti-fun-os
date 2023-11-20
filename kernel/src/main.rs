#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate bootloader_api;
extern crate lazy_static;

mod gdt;
mod interrupts;
mod logging;

use bootloader_api::info::MemoryRegionKind;
use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use log::{error, info};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    logging::init();

    info!("Entered kernel with boot info: {boot_info:?}");

    print_mem(&boot_info);

    gdt::init();
    interrupts::init_idt();

    unsafe {
        *(0xdeadbeef as *mut u8) = 42;
    };

    panic!("End of main!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info}");
    loop {}
}

fn print_mem(boot_info: &BootInfo) {
    for region in boot_info.memory_regions.iter() {
        info!("region: {region:?}");
    }

    let mut start: u64 = 0;
    let mut size: u64 = 0;

    for region in boot_info.memory_regions.iter() {
        if let MemoryRegionKind::Usable = region.kind {
            if start > 0 && start + size == region.start {
                size += region.end - region.start;
            } else {
                if size > 0 {
                    info!("usable region: start={start}, size={size}");
                }

                start = region.start;
                size = region.end - region.start;
            }
        } else {
            if size > 0 {
                info!("usable region: start={start}, size={size}");
            }

            start = 0;
            size = 0;
        }
    }

    if size > 0 {
        info!("usable region: start={start}, size={size}");
    }
}
