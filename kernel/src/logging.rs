use core::fmt::Write;
use spin::Mutex;
use uart_16550::SerialPort;

const SERIAL_IO_PORT: u16 = 0x3F8;

static serial_port_ref: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(SERIAL_IO_PORT) });

// let mut serial_port: SerialPort;

pub fn init() {
  let mut serial_port = serial_port_ref.lock();
  serial_port.init();
}

pub fn write(s: &str) {
  let mut serial_port = serial_port_ref.lock();
  serial_port.write_str(s).unwrap();
}
