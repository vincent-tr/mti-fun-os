#![no_std]
#![no_main]

use libruntime::kobject;
use log::{debug, info};

extern crate alloc;
extern crate libruntime;

mod rtc;

#[no_mangle]
pub fn main() -> i32 {
    info!("Time server started");

    let rtc_clock = rtc::read_rtc();
    info!("Current RTC time: {}", rtc_clock);

    loop {
        libruntime::timer::sleep(libruntime::timer::Duration::from_seconds(1));
    }
}
