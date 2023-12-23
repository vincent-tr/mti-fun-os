#![no_std]
#![no_main]

mod syscalls;

use core::{panic::PanicInfo, arch::asm};

use syscalls::{SyscallNumber, syscall3};

#[repr(usize)]
enum Level {
    Error = 1,
    Warn,
    Info,
    Debug,
    Trace,
}

fn log(level: Level, message: &str) {
    unsafe {
        syscall3(
            SyscallNumber::Log,
            level as usize,
            message.as_ptr() as usize,
            message.len(),
        )
    };
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        log(Level::Info, "test");
    }

    loop {}
}

#[inline]
fn debugbreak() {
    unsafe {
        asm!("int3", options(nomem, nostack));
    }
}

#[inline]
fn page_fault() {
    let ptr = 0x42 as *mut u8;
    unsafe { *ptr = 42 };
}

#[allow(unconditional_panic)]
#[inline]
fn div0() {
    // div / 0
    let _ = 42 / 0;
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
    //error!("PANIC: {info}");
}
