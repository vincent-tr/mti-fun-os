use core::{hint::unreachable_unchecked, panic::PanicInfo};
use libsyscalls::process;
use log::error;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info}");

    // Note: if case we failed exit, we cannot do much more.
    let _ = process::exit();
    unsafe { unreachable_unchecked() }
}
