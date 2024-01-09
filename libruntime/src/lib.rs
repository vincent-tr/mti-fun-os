#![no_std]

extern crate alloc;

mod allocator;
pub mod kobject;
mod logging;
mod panic;

pub fn init() {
    logging::init();
}
