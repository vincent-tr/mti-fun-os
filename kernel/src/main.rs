#![no_std]
#![no_main]

use kernel::{gdt, hlt_loop, interrupts, logging, println};

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    logging::init();
    gdt::init();
    interrupts::init();

    println!("Hello World!");

    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    x86_64::instructions::interrupts::disable();
    hlt_loop();
}
