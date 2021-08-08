#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(common::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod common;

use kernel::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {

    test_main();
    
    loop {}
}

#[test_case]
fn test_println() {
    println!("test_println output");
}
