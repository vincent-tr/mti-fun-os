#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(common::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod common;

use kernel::{hlt_loop, logging, print};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    logging::init();

    test_main();

    hlt_loop();
}

#[test_case]
fn test_print() {
    print!("test_print output");
}
