use core::fmt::{Debug, Formatter, Result};

use core::str;
/// Parameters to create a thread
#[repr(C)]
#[derive(Debug)]
pub struct ThreadCreationParameters {
    pub process_handle: u64,
    pub privileged: bool,
    pub priority: ThreadPriority,
    pub entry_point: usize,
    pub stack_top: usize,
    pub arg: usize,
    pub tls: usize,
}

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
#[repr(C)]
pub struct ThreadInfo {
    pub tid: u64,
    pub pid: u64,
    pub name: [u8; Self::NAME_LEN], // if name len == 0 then there is no name
    pub priority: ThreadPriority,
    pub privileged: bool,
    pub state: ThreadState,
    pub ticks: usize,
}

impl ThreadInfo {
    pub const NAME_LEN: usize = 128;
}

impl Debug for ThreadInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ThreadInfo")
            .field("tid", &self.tid)
            .field("pid", &self.pid)
            .field(
                "name",
                &format_args!(
                    "{}",
                    if self.name[0] != 0 {
                        unsafe { str::from_utf8_unchecked(&self.name) }
                    } else {
                        "<None>"
                    }
                ),
            )
            .field("priority", &self.priority)
            .field("privileged", &self.privileged)
            .field("state", &self.state)
            .field("ticks", &self.ticks)
            .finish()
    }
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

/// Context of the thread.
#[repr(C)]
#[derive(Debug)]
pub struct ThreadContext {
    pub rax: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rbx: usize,
    pub rsi: usize,
    pub rdi: usize,
    /// Stack pointer
    pub rsp: usize,
    pub rbp: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,

    /// Next instruction pointer
    pub instruction_pointer: usize,

    /// CPU flags
    pub cpu_flags: usize,

    // FS base value (used for TLS)
    pub tls: usize,
}

#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum ThreadContextRegister {
    Rax = 1,
    Rcx,
    Rdx,
    Rbx,
    Rsi,
    Rdi,
    /// Stack pointer
    Rsp,
    Rbp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,

    /// Next instruction pointer
    InstructionPointer,

    /// CPU flags
    CpuFlags,

    // FS base value (used for TLS)
    TLS,
}
