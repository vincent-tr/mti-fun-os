use alloc::boxed::Box;
pub use syscalls::{ProcessEventType, ThreadEventType};

pub fn notify_process(pid: u64, r#type: ProcessEventType) {}

pub fn notify_thread(pid: u64, r#type: ThreadEventType) {}

