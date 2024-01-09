#![no_std]

extern crate alloc;

mod allocator;
mod logging;
mod panic;

pub fn init() {
    logging::init();
}
