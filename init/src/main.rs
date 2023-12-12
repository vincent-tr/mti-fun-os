#![no_std]
#![no_main]

mod syscalls;

use core::panic::PanicInfo;

use syscalls::syscall0;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        syscall0(1);
        syscall0(2);
    }
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
    //error!("PANIC: {info}");
}
