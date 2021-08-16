#![no_std]
#![feature(abi_x86_interrupt)]

use bootloader::BootInfo;

pub mod error;
pub mod gdt;
pub mod interrupts;
pub mod logging;
pub mod memory;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn init(boot_info: &'static BootInfo) {
    logging::init();
    gdt::init();
    interrupts::init();
    memory::init(boot_info);
}
