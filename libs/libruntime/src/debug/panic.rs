use core::{
    fmt,
    hint::unreachable_unchecked,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

use libsyscalls::process;
use log::error;

use super::StackTrace;

static PANICKING: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    do_panic(info);
}

fn do_panic(info: &PanicInfo) -> ! {
    // Check for re-entrancy (panic within panic handler)
    if PANICKING.swap(true, Ordering::SeqCst) {
        // We're already panicking - this is a double panic!
        // Don't try to do anything fancy, just exit immediately
        error!(
            "DOUBLE PANIC - panic handler panicked! - {}",
            info.message()
        );
        let _ = process::exit();
        unsafe { unreachable_unchecked() }
    }

    let stacktrace = StackTrace::capture();
    error!("PANIC: {}", PanicDisplay::new(info, stacktrace));

    // Note: in case we failed exit, we cannot do much more.
    let _ = process::exit();
    unsafe { unreachable_unchecked() }
}

struct PanicDisplay<'a> {
    info: &'a PanicInfo<'a>,
    stacktrace: StackTrace,
}

impl<'a> PanicDisplay<'a> {
    pub fn new(info: &'a PanicInfo<'a>, stacktrace: StackTrace) -> Self {
        Self { info, stacktrace }
    }
}

impl fmt::Display for PanicDisplay<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.info.message().fmt(formatter)?;

        formatter.write_str("\n")?;

        let mut skipping = true;

        for frame in self.stacktrace.iter() {
            if skipping {
                if let Some(info) = frame.location()
                    && let Some(function) = info.function_name()
                {
                    if function.contains("rust_begin_unwind") {
                        // this marks the end of the stack inner panic handling stuff
                        skipping = false;
                    }

                    continue;
                } else {
                    // we loose info, do not skip from here
                    skipping = false;
                }
            }

            formatter.write_str("  at ")?;
            if let Some(info) = frame.location() {
                if let Some(function) = info.function_name() {
                    formatter.write_str(&function)?;
                } else {
                    formatter.write_str("???")?;
                }
                if let Some(location) = info.source_location() {
                    formatter.write_str(" - ")?;
                    location.fmt(formatter)?;
                }
            } else {
                formatter.write_fmt(format_args!("0x{0:016X}", frame.address()))?;
            }

            formatter.write_str("\n")?;
        }

        Ok(())
    }
}
