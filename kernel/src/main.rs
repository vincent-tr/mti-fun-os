#![no_std]
#![no_main]

extern crate bootloader_api;
extern crate lazy_static;

mod logging;

use bootloader_api::info::MemoryRegionKind;
use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use log::{error, info};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    logging::init();

    info!("Entered kernel with boot info: {boot_info:?}");


    panic!("End of main!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info}");
    loop {}
}
