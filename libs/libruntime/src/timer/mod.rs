mod duration;

pub use duration::Duration;

use crate::kobject;

/// Sleep for the specified duration
pub fn sleep(duration: Duration) {
    let now = kobject::Timer::now().expect("failed to get current time");
    let timer = kobject::Timer::create(0).expect("failed to create timer");
    timer
        .arm(now + duration.as_u64())
        .expect("failed to arm timer");
    timer
        .blocking_receive()
        .expect("failed to receive timer event");
}

pub async fn async_sleep(duration: Duration) {
    let now = kobject::Timer::now().expect("failed to get current time");
    let timer = kobject::Timer::create(0).expect("failed to create timer");
    timer
        .arm(now + duration.as_u64())
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
