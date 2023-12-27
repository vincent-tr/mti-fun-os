/// Thread priority
#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ThreadPriority {
    Idle = 1,
    Lowest,
    BelowNormal,
    Normal,
    AboveNormal,
    Highest,
    TimeCritical,
}
