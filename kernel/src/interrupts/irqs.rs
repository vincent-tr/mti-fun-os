use log::info;
use x86_64::structures::idt::InterruptStackFrame;

use crate::devices;

// TODO: Properly push context
pub extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
  info!(".");

  devices::notify_end_of_interrupt(devices::IRQ0);
}
