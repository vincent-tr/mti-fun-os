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

/// State of a thread
#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ThreadState {
    /// The thread is currently executing.
    ///
    /// When in kernel mode, this is the one that is currently configured as current, and on the interrupt stack
    Executing = 1,

    /// This thread is ready to be scheduled
    Ready,

    /// This thread is sleeping, waiting for something
    Waiting,

    /// This thread got an error (eg: page fault).
    ///
    /// It can be resumed after the error has been solved.
    Error,

    /// This thread has been terminated
    Terminated,
}

/// Thread information
#[repr(C, packed)]
#[derive(Debug)]
pub struct ThreadInfo {
    pub tid: u64,
    pub pid: u64,
    pub priority: ThreadPriority,
    pub state: ThreadState,
    pub ticks: usize,
}
