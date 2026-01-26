use super::{exit, init};

extern "Rust" {
    fn main();
}

/// Program entry point
#[no_mangle]
extern "C" fn _start(arg: usize) -> ! {
    // Note: the entry thread is not registered in the thread GC.
    // When this thread exits, the process will exit.

    init();

    // TODO: fetch context (env, args)
    unsafe { main() };
    // TODO: report exit code

    exit();
}
