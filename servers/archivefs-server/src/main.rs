#![no_std]
#![no_main]

use log::info;

extern crate alloc;
extern crate libruntime;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    info!("ArchiveFS server started");

    // TODO: Implement archivefs server logic

    0
}
