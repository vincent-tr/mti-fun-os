use core::{fmt, hint::unreachable_unchecked, panic::PanicInfo};

use libsyscalls::process;
use log::error;

use super::StackTrace;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    do_panic(info);
}

fn do_panic(info: &PanicInfo) -> ! {
    // TODO: check for re-entrancy (if the panic handler panics)

    let stacktrace = StackTrace::capture();
    error!("PANIC: {}", PanicDisplay::new(info, stacktrace));

    // Note: if case we failed exit, we cannot do much more.
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
        // From PanicInfo source
        if let Some(message) = self.info.message() {
            formatter.write_fmt(*message)?;
        } else if let Some(payload) = self.info.payload().downcast_ref::<&'static str>() {
            formatter.write_str(payload)?;
        }
        // The payload is a String when `std::panic!` is called with multiple arguments,
        // but in that case the message is also available.

        formatter.write_str("\n")?;

        let mut skipping = true;

        for frame in self.stacktrace.iter() {
            if skipping {
                if let Some(info) = frame.location()
                    && let Some(function) = info.function_name()
                {
                    if function == "rust_begin_unwind" {
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
