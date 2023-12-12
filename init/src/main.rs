#![no_std]
#![no_main]

mod syscalls;

use core::panic::PanicInfo;

use syscalls::syscall0;

const SYSCALL_EXIT: usize = 1;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        syscall0(SYSCALL_EXIT);
        syscall0(2);
    }
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
    //error!("PANIC: {info}");
}
