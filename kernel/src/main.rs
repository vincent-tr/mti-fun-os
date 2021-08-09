#![no_std]
#![no_main]

use kernel::{hlt_loop, interrupts, logging, println};

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    logging::init();
    interrupts::init();

    println!("Hello World!");

    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}
