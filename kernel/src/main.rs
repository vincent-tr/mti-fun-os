#![no_std]
#![no_main]

mod logging;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    logging::init();

    println!("Hello World!zzz");

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
