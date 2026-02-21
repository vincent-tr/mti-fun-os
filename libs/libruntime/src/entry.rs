use crate::process;

use super::{exit, init};

unsafe extern "Rust" {
    fn main() -> i32;
}

/// Program entry point
#[unsafe(no_mangle)]
extern "C" fn _start(_arg: usize) -> ! {
    // Note: the entry thread is not registered in the thread GC.
    // When this thread exits, the process will exit.

    init();

    // Force process init so that we get the symbols (for panic)
    #[cfg(feature = "init-process")]
    init_process();

    let exit_code = unsafe { main() };
    process::SelfProcess::get().set_exit_code(exit_code);

    exit();
}

#[cfg(feature = "init-process")]
fn init_process() {
    process::SelfProcess::get();
}
