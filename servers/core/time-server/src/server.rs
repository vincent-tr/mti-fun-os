use libruntime::{
    kobject,
    time::{
        DateTime, Duration,
        iface::{TimeServer, TimeServerError},
    },
};

use log::debug;

use crate::rtc;

/// Time Server
#[derive(Debug)]
pub struct Server {
    boot_monotonic: Duration,
    boot_wall: DateTime,
}

impl Server {
    pub fn new() -> Self {
        // first read rtc as it's slower
        let boot_wall = rtc::read_rtc();
        let boot_monotonic = Self::now_monotonic();

        debug!(
            "Boot time: wall = {}, monotonic = {}",
            boot_wall, boot_monotonic
        );

        Self {
            boot_wall,
            boot_monotonic,
        }
    }

    fn now_monotonic() -> Duration {
        Duration::nanoseconds(kobject::Timer::now().expect("failed to get current time") as i64)
    }
}

impl TimeServer for Server {
    type Error = TimeServerError;

    fn get_wall_time(&self, _sender_id: u64) -> Result<DateTime, Self::Error> {
        let now_monotonic = Self::now_monotonic();
        let elapsed = now_monotonic - self.boot_monotonic;
        let now_wall = self.boot_wall + elapsed;
        Ok(now_wall)
    }
}
