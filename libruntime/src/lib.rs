#![no_std]

mod allocator;
mod logging;
mod panic;

pub fn init() {
    logging::init();
}
