pub mod iface;

pub use ::time::Duration;
pub use ::time::UtcDateTime as DateTime;
use lazy_static::lazy_static;

use crate::kobject;

/// Sleep for the specified duration
pub fn sleep(duration: Duration) {
    let now = get_monotonic_time();
    let timer = kobject::Timer::create(0).expect("failed to create timer");
    timer
        .arm((now + duration).whole_nanoseconds() as u64)
        .expect("failed to arm timer");
    timer
        .blocking_receive()
        .expect("failed to receive timer event");
}

/// Asynchronously sleep for the specified duration
pub async fn async_sleep(duration: Duration) {
    let now = get_monotonic_time();
    let timer = kobject::Timer::create(0).expect("failed to create timer");
    timer
        .arm((now + duration).whole_nanoseconds() as u64)
        .expect("failed to arm timer");

    let res = loop {
        crate::r#async::wait(&timer).await;

        match timer.receive() {
            Err(crate::kobject::Error::ObjectNotReady) => {
                continue;
            }
            other => {
                break other;
            }
        }
    };

    res.expect("failed to receive timer event");
}

/// Get the current monotonic time as a Duration
pub fn get_monotonic_time() -> Duration {
    let timestamp = kobject::Timer::now().expect("failed to get current time");
    Duration::nanoseconds(timestamp as i64)
}

/// Get the current wall clock time
pub fn get_wall_time() -> DateTime {
    TIME_CLIENT
        .get_wall_time()
        .expect("failed to get wall time")
}

lazy_static! {
    static ref TIME_CLIENT: iface::Client = iface::Client::new();
}
