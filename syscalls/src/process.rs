/// Thread information
#[repr(C, packed)]
#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u64,
    pub thread_count: usize,
    pub mapping_count: usize,
    pub handle_count: usize,
}
