/// Timer event
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TimerEvent {
    /// Id provided when creating the timer
    pub id: u64,

    /// Current monotonic clock time
    pub now: u64,
}
