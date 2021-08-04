#![no_std]
#![no_main]

mod logging;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    logging::init();

    println!("Hello World!");

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
