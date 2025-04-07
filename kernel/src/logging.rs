use core::fmt::Write;
use lazy_static::lazy_static;
use log::{Metadata, Record};

struct KernelLogger;

lazy_static! {
    static ref SERIAL1: spin::Mutex<uart_16550::SerialPort> = {
        let mut port = unsafe { uart_16550::SerialPort::new(0x3F8) };
        port.init();
        spin::Mutex::new(port)
    };
}

impl log::Log for KernelLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        //metadata.level() <= Level::Info
        return true;
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut serial = SERIAL1.lock();
            let _ = writeln!(serial, "{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: KernelLogger = KernelLogger;

pub fn init() {
    // Note: if set logger fails, there is not much we can do since panic also use the logger
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Trace); // Trace is very verbose
}
