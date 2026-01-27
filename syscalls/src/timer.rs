/// Timer event
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TimerEvent {
    /// Id provided when creating the timer
    pub id: u64,

    /// Current monotonic tick count
    pub tick: u64,
}
