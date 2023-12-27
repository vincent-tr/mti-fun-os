/// List of syscall numbers
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallNumber {
    Log = 1,
    Close,
    Duplicate,
    ProcessOpenSelf,
    ProcessCreate,
    ProcessMMap,
    ProcessMUnmap,
    ProcessMProtect,

    InitSetup,
}
