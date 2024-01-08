/// Thread priority
#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
pub enum ThreadPriority {
    Idle = 1,
    Lowest,
    BelowNormal,
    #[default]
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

#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum Exception {
    DivideError = 1,

    Debug,

    /// Cannot happen in userland
    NonMaskableInterrupt,

    Breakpoint,

    Overflow,

    BoundRangeExceeded,

    InvalidOpcode,

    DeviceNotAvailable,

    /// Cannot happen in userland
    DoubleFault,

    /// Cannot happen in userland
    InvalidTSS,

    /// Cannot happen in userland
    SegmentNotPresent(usize),

    StackSegmentFault(usize),

    GeneralProtectionFault(usize),

    /// Second parameter is value of CR2: accessed address
    PageFault(usize, usize),

    X87FloatingPoint,

    AlignmentCheck,

    /// Cannot happen in userland
    MachineCheck,

    SimdFloatingPoint,

    /// Cannot happen in userland
    Virtualization,

    CpProtectionException(usize),

    /// Cannot happen in userland
    HvInjectionException,

    /// Cannot happen in userland
    VmmCommunicationException(usize),

    /// Cannot happen in userland
    SecurityException(usize),
}
