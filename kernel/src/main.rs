#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

use kernel::{gdt, hlt_loop, interrupts, logging, memory, println};

entry_point!(kernel_main);

#[no_mangle]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    logging::init();
    gdt::init();
    interrupts::init();
    memory::init(boot_info);

    println!("Hello World!");

    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    x86_64::instructions::interrupts::disable();
    hlt_loop();
}
