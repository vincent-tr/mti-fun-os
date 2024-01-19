use alloc::fmt::format;
use log::{Metadata, Record};

struct InitLogger;

// https://stackoverflow.com/questions/50200268/how-can-i-use-the-format-macro-in-a-no-std-environment
mod write_to {
    use core::cmp::min;
    use core::fmt;

    pub struct WriteTo<'a> {
        buffer: &'a mut [u8],
        // on write error (i.e. not enough space in buffer) this grows beyond
        // `buffer.len()`.
        used: usize,
    }

    impl<'a> WriteTo<'a> {
        pub fn new(buffer: &'a mut [u8]) -> Self {
            WriteTo { buffer, used: 0 }
        }

        pub fn as_str(self) -> Option<&'a str> {
            if self.used <= self.buffer.len() {
                // only successful concats of str - must be a valid str.
                use core::str::from_utf8_unchecked;
                Some(unsafe { from_utf8_unchecked(&self.buffer[..self.used]) })
            } else {
                None
            }
        }
    }

    impl<'a> fmt::Write for WriteTo<'a> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            if self.used > self.buffer.len() {
                return Err(fmt::Error);
            }
            let remaining_buf = &mut self.buffer[self.used..];
            let raw_s = s.as_bytes();
            let write_num = min(raw_s.len(), remaining_buf.len());
            remaining_buf[..write_num].copy_from_slice(&raw_s[..write_num]);
            self.used += raw_s.len();
            if write_num < raw_s.len() {
                Err(fmt::Error)
            } else {
                Ok(())
            }
        }
    }

    pub fn show<'a>(buffer: &'a mut [u8], args: fmt::Arguments) -> Result<&'a str, fmt::Error> {
        let mut w = WriteTo::new(buffer);
        fmt::write(&mut w, args)?;
        w.as_str().ok_or(fmt::Error)
    }
}

impl log::Log for InitLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        //metadata.level() <= Level::Info
        return true;
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // First try to format with buffer on stack, then if too big do from heap
            if !Self::try_log_from_stack(record) {
                Self::log_from_heap(record);
            }
        }
    }

    fn flush(&self) {}
}

impl InitLogger {
    fn try_log_from_stack(record: &Record) -> bool {
        let mut buf: [u8; 1024] = [0u8; 1024];

        match write_to::show(&mut buf, *record.args()) {
            Ok(message) => {
                Self::syscall(record, message);
                true
            }
            Err(_) => false,
        }
    }

    fn log_from_heap(record: &Record) {
        let message = format(*record.args());
        Self::syscall(record, &message);
    }

    fn syscall(record: &Record, message: &str) {
        // If logging fails, there is not much we can do...
        let _ = libsyscalls::log(record.level(), message);
    }
}

static LOGGER: InitLogger = InitLogger;

pub fn init() {
    // Note: if set logger fails, there is not much we can do since panic also use the logger
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Debug); // Trace is very verbose
}
