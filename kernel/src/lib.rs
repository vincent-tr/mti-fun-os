#![no_std]
#![feature(abi_x86_interrupt)]

pub mod gdt;
pub mod interrupts;
pub mod logging;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
