/// Handle type
#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum HandleType {
    Invalid,
    MemoryObject,
    Process,
    Thread,
    PortSender,
    PortReceiver,
    PortRange,
    ProcessListener,
    ThreadListener,
    Timer,
}
