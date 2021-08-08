#![no_std]
#![no_main]

use kernel::logging;
use kernel::println;

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
