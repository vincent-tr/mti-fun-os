#![no_std]
#![no_main]

use log::info;

extern crate alloc;
extern crate libruntime;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    info!("PCI server started");

    // TODO: Implement PCI device enumeration and management

    info!("PCI server initialized");

    0
}
