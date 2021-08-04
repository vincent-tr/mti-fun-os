use core::fmt;
use spin::Mutex;
use uart_16550::SerialPort;

const SERIAL_IO_PORT: u16 = 0x3F8;

static WRITER: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(SERIAL_IO_PORT) });

pub fn init() {
    WRITER.lock().init();
}

/// Like the `print!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::logging::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
