mod timer;
mod timers;

pub use timer::Timer;

use core::sync::atomic::{AtomicU64, Ordering};

// Interrupt period = 10ms
const TICK_PER_SECOND: u64 = 100;
const NS_PER_SECOND: u64 = 1_000_000_000;
const NS_PER_TICK: u64 = NS_PER_SECOND / TICK_PER_SECOND;

static MONOTONIC_NS: AtomicU64 = AtomicU64::new(0);

/// Called on each timer tick
pub fn tick() {
    let before = MONOTONIC_NS.fetch_add(NS_PER_TICK, Ordering::SeqCst);

    // before is the previous value, rebuild the current time
    let now = before + NS_PER_TICK;

    timers::TIMERS.tick(now);
}

pub fn now() -> u64 {
    MONOTONIC_NS.load(Ordering::SeqCst)
}
