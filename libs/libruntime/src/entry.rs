use crate::process;

use super::{exit, init};

extern "Rust" {
    fn main() -> i32;
}

/// Program entry point
#[no_mangle]
extern "C" fn _start(_arg: usize) -> ! {
    // Note: the entry thread is not registered in the thread GC.
    // When this thread exits, the process will exit.

    init();

    let exit_code = unsafe { main() };
    process::SelfProcess::get().set_exit_code(exit_code);

    exit();
}
