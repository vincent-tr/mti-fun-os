/// Process event
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ProcessEvent {
    /// PID of the process this event occurs on
    pub pid: u64,

    /// Type of event
    pub r#type: ProcessEventType,
}

/// Process event type
#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ProcessEventType {
    /// Process has been created.
    Created = 1,

    /// Process has terminated: last thread terminated, all handles has been closed, no more execution will happen in the process.
    Terminated,

    /// Process has been deleted: it does not exist anymore in the system.
    Deleted,
}

/// Process event
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ThreadEvent {
    /// TID of the thread this event occurs on
    pub tid: u64,

    /// Type of event
    pub r#type: ThreadEventType,
}

/// Thread event type
#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ThreadEventType {
    /// Thread has been created.
    Created = 1,

    /// An error occured on the thread. It is in error state.
    Error,

    /// Thread has been resumed after an error.
    Resumed,

    /// Thread has terminated: no more execution will happen in the thread.
    Terminated,

    /// Thread has been deleted: it does not exist anymore in the system
    Deleted,
}
